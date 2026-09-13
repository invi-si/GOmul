import type { AppMetadata } from './app_library_store';

export interface CatalogGame {
  id: string;
  title: string;
  carrier: string;
  source: string;
  thumbnail: string | null;
}
interface Attachment { name: string; index: number }
export interface CatalogGroup { title: string; versions: CatalogGame[] }
const normalizeTitle = (text: string) => text.normalize('NFKC').toLocaleLowerCase('ko').replace(/\s+/g, '');

export function groupGames(games: CatalogGame[]): CatalogGroup[] {
  const groups = new Map<string, CatalogGroup>();
  for (const game of games) {
    const key = normalizeTitle(game.title);
    const group = groups.get(key);
    if (group) group.versions.push(game);
    else groups.set(key, {title:game.title, versions:[game]});
  }
  return [...groups.values()];
}


export function matchesGame(game: Pick<CatalogGame, 'title' | 'carrier'>, query: string, carrier: string): boolean {
  const normalize = normalizeTitle;
  return (!carrier || game.carrier === carrier) && normalize(game.title).includes(normalize(query));
}

export async function initializeCatalog(launch: (app: AppMetadata, selection: {game: string; index: number}) => Promise<void>, onLoaded?: (games:CatalogGame[])=>void): Promise<void> {
  const list = document.getElementById('catalog-list')!;
  const search = document.getElementById('catalog-search') as HTMLInputElement;
  const carrier = document.getElementById('catalog-carrier') as HTMLSelectElement;
  const count = document.getElementById('catalog-count')!;
  const status = document.getElementById('catalog-status')!;
  const loading = document.getElementById('catalog-loading')!;
  const loadingImage = document.getElementById('loading-image') as HTMLImageElement;
  const showLoadingArt = (title: string, thumbnail: string | null) => {
    document.getElementById('loading-title')!.textContent = title;
    document.getElementById('loading-fallback')!.textContent = title.slice(0,1);
    loadingImage.hidden = !thumbnail;
    loadingImage.onerror = () => { loadingImage.hidden = true; };
    if (thumbnail) loadingImage.src = thumbnail;
    else loadingImage.removeAttribute('src');
  };
  const retry = document.getElementById('catalog-retry') as HTMLButtonElement;
  const cancel = document.getElementById('catalog-cancel') as HTMLButtonElement;
  const dialog = document.getElementById('variant-dialog') as HTMLDialogElement;
  const variants = document.getElementById('variant-list')!;
  const ktfNotice = document.getElementById('ktf-notice-dialog') as HTMLDialogElement;
  const dismissKtfNotice = document.getElementById('ktf-notice-dismiss') as HTMLInputElement;
  let ktfNoticeHidden = false;
  try { ktfNoticeHidden = localStorage.getItem('gomul-hide-ktf-notice') === '1'; } catch { /* Storage may be disabled. */ }
  const confirmKtf = (): Promise<boolean> => {
    if (ktfNoticeHidden) return Promise.resolve(true);
    return new Promise(resolve => {
      ktfNotice.returnValue = '';
      dismissKtfNotice.checked = false;
      ktfNotice.addEventListener('close', () => {
        const proceed = ktfNotice.returnValue === 'continue';
        if (proceed && dismissKtfNotice.checked) {
          ktfNoticeHidden = true;
          try { localStorage.setItem('gomul-hide-ktf-notice', '1'); } catch { /* Retain the choice for this page. */ }
        }
        resolve(proceed);
      }, {once:true});
      ktfNotice.showModal();
    });
  };
  const sort = document.getElementById('catalog-sort') as HTMLSelectElement;
  let games: CatalogGame[] = [];
  let busy = false;
  let controller: AbortController | undefined;

  // Keep spare HTTP connections for game requests. Native lazy loading can
  // otherwise start many thumbnails at once and occupy every browser slot.
  let activeThumbnails = 0;
  let thumbnailQueue: HTMLImageElement[] = [];
  const pumpThumbnails = () => {
    while (!busy && activeThumbnails < 2 && thumbnailQueue.length) {
      const image = thumbnailQueue.shift()!;
      if (!image.isConnected) continue;
      activeThumbnails++;
      const finish = (failed: boolean) => {
        image.onload = null;
        image.onerror = null;
        if (failed) image.remove();
        activeThumbnails--;
        pumpThumbnails();
      };
      image.onload = () => finish(false);
      image.onerror = () => finish(true);
      image.src = image.dataset.thumbnail!;
    }
  };
  const thumbnails = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        thumbnails.unobserve(entry.target);
        thumbnailQueue.push(entry.target as HTMLImageElement);
      }
    }
    pumpThumbnails();
  }, {rootMargin: '300px'});

  const message = (text: string, error = false) => {
    status.textContent = text;
    document.getElementById('loading-status')!.textContent = text;
    status.classList.toggle('error', error);
  };
  const response = async (path: string): Promise<Response> => {
    const result = await fetch(path, { signal: controller?.signal });
    if (!result.ok) {
      const body = await result.json().catch(() => null) as {error?: string} | null;
      throw new Error(body?.error || `요청 실패 (${result.status})`);
    }
    return result;
  };

  const choose = <T,>(items: T[], label: (item: T) => string, title: string, description: string): Promise<T | undefined> => new Promise(resolve => {
    variants.replaceChildren();
    document.getElementById('variant-title')!.textContent = title;
    document.getElementById('variant-description')!.textContent = description;
    const closed = () => resolve(undefined);
    dialog.addEventListener('close', closed, { once: true });
    for (const item of items) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'secondary-command';
      button.textContent = label(item);
      button.onclick = () => {
        dialog.removeEventListener('close', closed);
        dialog.close();
        resolve(item);
      };
      variants.append(button);
    }
    dialog.showModal();
  });

  const play = async (group: CatalogGroup) => {
    if (busy) return;
    busy = true;
    controller = new AbortController();
    retry.hidden = true;
    cancel.hidden = false;
    list.setAttribute('aria-busy', 'true');
    list.inert = true;
    showLoadingArt(group.title, group.versions.find(version => version.thumbnail)?.thumbnail || null);
    loading.hidden = false;
    message(`${group.title} · ${group.versions.length > 1 ? '실행할 버전을 선택하세요.' : '게임을 준비하는 중…'}`);
    try {
      const game = group.versions.length === 1 ? group.versions[0] : await choose(group.versions,
        version => `${version.carrier} 버전`, '어떤 버전으로 실행할까요?', group.title);
      if (!game || controller.signal.aborted) { message('실행을 취소했습니다.'); return; }
      if (game.carrier === 'KTF' && !await confirmKtf()) { message('실행을 취소했습니다.'); return; }
      if (controller.signal.aborted) { message('실행을 취소했습니다.'); return; }
      showLoadingArt(`${game.title} · ${game.carrier}`, game.thumbnail);
      message(`${game.title} · ${game.carrier} 첨부파일 확인 중…`);
      const files = await (await response(`/api/game/${game.id}/files`)).json() as Attachment[];
      if (!files.length) throw new Error('이 게시물에는 바로 실행할 수 있는 ZIP/JAR/ALZ 첨부파일이 없습니다. 원문을 확인하세요.');
      const file = files.length === 1 ? files[0] : await choose(files, file => file.name, '실행할 파일 선택', '이 버전에는 여러 첨부파일이 있습니다.');
      if (!file || controller.signal.aborted) { message('실행을 취소했습니다.'); return; }
      message(`${game.title} · 파일 준비 및 게임 시작 중…`);
      cancel.hidden = true;
      await launch({ id: `${game.carrier}-${game.id}`, title: game.title, filename: file.name, addedAt: Date.now() }, {game:game.id,index:file.index});
      message(`${game.title} · 종료했습니다. 다른 게임을 선택하세요.`);
    } catch (error) {
      if (controller?.signal.aborted) message('다운로드를 취소했습니다.');
      else message(error instanceof Error ? error.message : String(error), true);
    } finally {
      busy = false;
      controller = undefined;
      cancel.hidden = true;
      list.removeAttribute('aria-busy');
      list.inert = false;
      loading.hidden = true;
      pumpThumbnails();
    }
  };
  cancel.onclick = () => { controller?.abort(); if (dialog.open) dialog.close(); if (ktfNotice.open) ktfNotice.close(); };

  const render = () => {
    const visible = groupGames(games.filter(game => matchesGame(game, search.value, carrier.value)));
    visible.sort((a,b) => a.title.localeCompare(b.title,'ko') * (sort.value === 'desc' ? -1 : 1));
    count.textContent = `${visible.length}개 게임 · 전체 ${groupGames(games).length}개 / ${games.length}개 버전`;
    thumbnails.disconnect();
    thumbnailQueue = [];
    list.replaceChildren();
    if (!visible.length) {
      const empty = document.createElement('p');
      empty.className = 'catalog-empty';
      empty.textContent = '검색 결과가 없습니다. 다른 이름이나 통신사를 선택하세요.';
      list.append(empty);
    }
    for (const group of visible) {
      const game = group.versions.find(version => version.thumbnail) || group.versions[0];
      const carriers = [...new Set(group.versions.map(version => version.carrier))].join(' · ');
      const row = document.createElement('li');
      row.className = 'catalog-row';
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'catalog-play';
      button.setAttribute('aria-label', `${group.title} ${carriers} 실행`);
      const art = document.createElement('span');
      art.className = 'catalog-art';
      art.textContent = game.title.slice(0, 1);
      if (game.thumbnail) {
        const image = document.createElement('img');
        image.dataset.thumbnail = game.thumbnail;
        image.alt = '';
        image.fetchPriority = 'low';
        image.decoding = 'async';
        art.append(image);
        thumbnails.observe(image);
      }
      const title = document.createElement('strong');
      title.textContent = game.title;
      const badge = document.createElement('span');
      badge.className = 'carrier-badge';
      badge.textContent = carriers;
      const arrow = document.createElement('span');
      arrow.className = 'catalog-arrow';
      arrow.textContent = '▶';
      arrow.setAttribute('aria-hidden', 'true');
      button.append(art, title, badge, arrow);
      button.onclick = () => void play(group);
      const sources = document.createElement('div');
      sources.className = 'catalog-sources';
      for (const version of group.versions) {
        const source = document.createElement('a');
        source.href = version.source;
        source.target = '_blank';
        source.rel = 'noopener noreferrer';
        source.className = 'catalog-source';
        source.textContent = `${version.carrier} ↗`;
        source.setAttribute('aria-label', `${version.title} ${version.carrier} 원문 보기`);
        sources.append(source);
      }
      row.append(button, sources);
      list.append(row);
    }
  };
  search.addEventListener('input', render);
  carrier.addEventListener('change', render);
  sort.addEventListener('change', render);
  for (const mode of ['grid', 'list']) {
    document.getElementById(`view-${mode}`)!.addEventListener('click', () => {
      list.dataset.view = mode;
      for (const other of ['grid', 'list']) document.getElementById(`view-${other}`)!.setAttribute('aria-pressed', String(mode === other));
    });
  }

  const load = async () => {
    retry.hidden = true;
    message('DubiGame에서 게임 목록을 불러오는 중… 처음에는 잠시 걸릴 수 있습니다.');
    try {
      games = await (await response('/api/catalog')).json() as CatalogGame[];
      games.sort((a, b) => a.title.localeCompare(b.title, 'ko') || a.carrier.localeCompare(b.carrier));
      onLoaded?.(games);
      render();
      message('게임을 선택하면 다운로드 후 실행합니다.');
    } catch (error) {
      message(error instanceof Error ? error.message : String(error), true);
      retry.hidden = false;
    }
  };
  retry.onclick = () => void load();
  await load();
}
