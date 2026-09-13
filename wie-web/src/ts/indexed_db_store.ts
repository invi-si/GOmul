const pendingWrites=new Set<Promise<void>>();
function trackWrite(write:Promise<void>):Promise<void> {
  const tracked=write.finally(()=>pendingWrites.delete(tracked));
  pendingWrites.add(tracked);return tracked;
}
export async function flushBrowserWrites():Promise<void> {
  await Promise.all([...pendingWrites]);
}

export class IndexedDBStore {
  private static readonly opened = new Map<string, Promise<IndexedDBStore>>();
  private db: IDBDatabase;
  private store_name: string;

  private constructor(db: IDBDatabase, store_name: string) {
    this.db = db;
    this.store_name = store_name;
  }

  public static open(db_name: string, store_name: string): Promise<IndexedDBStore> {
    const identity=JSON.stringify([db_name,store_name]);
    const existing=this.opened.get(identity);if(existing)return existing;
    const pending=new Promise<IndexedDBStore>((resolve, reject) => {
      const request = indexedDB.open(db_name);

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains(store_name)) {
          db.createObjectStore(store_name);
        }
      };

      request.onsuccess = (event) => {
        const db=(event.target as IDBOpenDBRequest).result;
        db.onversionchange=()=>{db.close();this.opened.delete(identity);};
        resolve(new IndexedDBStore(db, store_name));
      };

      request.onerror = (event) => {
        reject((event.target as IDBOpenDBRequest).error);
      };
    }).catch(error=>{this.opened.delete(identity);throw error;});
    this.opened.set(identity,pending);return pending;
  }

  public get_all_keys(): Promise<IDBValidKey[]> {
    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction(this.store_name, "readonly");
      const store = transaction.objectStore(this.store_name);
      const request = store.getAllKeys();

      request.onsuccess = () => {
        resolve(request.result);
      };

      request.onerror = () => {
        reject(request.error);
      };
    });
  }

  public get(key: IDBValidKey): Promise<Uint8Array | undefined> {
    return new Promise((resolve, reject) => {
      const transaction = this.db.transaction(this.store_name, "readonly");
      const store = transaction.objectStore(this.store_name);
      const request = store.get(key);

      request.onsuccess = () => {
        resolve(request.result as Uint8Array | undefined);
      };

      request.onerror = () => {
        reject(request.error);
      };
    });
  }

  public set(key: IDBValidKey, data: Uint8Array): Promise<void> {
    return trackWrite(new Promise<void>((resolve, reject) => {
      const transaction = this.db.transaction(this.store_name, "readwrite");
      const store = transaction.objectStore(this.store_name);
      const request = store.put(data, key);

      transaction.oncomplete = () => resolve();
      transaction.onabort = () => reject(transaction.error || new Error("Browser save transaction aborted"));

      request.onerror = () => {
        reject(request.error);
      };
    }));
  }

  public delete(key: IDBValidKey): Promise<void> {
    return trackWrite(new Promise<void>((resolve, reject) => {
      const transaction = this.db.transaction(this.store_name, "readwrite");
      const store = transaction.objectStore(this.store_name);
      const request = store.delete(key);

      transaction.oncomplete = () => resolve();
      transaction.onabort = () => reject(transaction.error || new Error("Browser save transaction aborted"));

      request.onerror = () => {
        reject(request.error);
      };
    }));
  }
}
