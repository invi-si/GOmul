# Super Action Hero 3: local setup

## In the Android app (alpha.3 and later)

Import your complete game ZIP with its companion data folder included. GOmul detects the LGT app identity and the savedata/locdata/ranker/it…/mk… files, installs them into that game’s private save storage, and launches it. A single enclosing package directory is accepted. GOmul also checks for supplied companion data when opening an already-imported game. It applies that bundle once, backing up any existing save before replacement. A local marker prevents repeatedly resetting later progress. If needed, an already-configured per-game phone identity is reused.

For separate data, hold the game in the library, choose **Import saved data**, then select its data ZIP or folder. Replacing existing progress requires confirmation and creates a local backup first.

If the package includes a `gomul.properties` text file with `phoneNumber=` followed by its required 11-digit emulated identity, setup is automatic. Otherwise GOmul asks for that number once. Obtain it from the supplier of your data; the APK does not bundle or guess an identity. After setup, the game reads its imported save normally—this is not a Quick Load snapshot and works without a Mac helper. The game’s own startup/menu screens still run.

No game/data download or patching is performed. You must supply the files. The alpha.2 import path was tested privately through the Android file picker: all nine companion files were installed byte-for-byte and the game reached its main menu without the authentication error.

## Optional protected startup on a Mac AVD

This procedure is for the tested LGT package and the native APK on a Mac-hosted Android Virtual Device. It creates a checkpoint locally. Nothing is downloaded or bundled by the setup tool.

1. Obtain your own complete game package, the accompanying data folder, and the emulated phone identity specified for that particular package. The outer ZIP must contain `app_info` and its application JAR at the root. Importing the JAR alone may lose its carrier/application identifiers.
2. Follow the build guide and install the checkpoint helper on an otherwise empty AVD. It prints a `bridge.json` path used below.
3. Import the supplied files, using your actual paths and required 11-digit identity:

   ```sh
   python3 tools/mac-checkpoints/setup.py import \
     --config "$HOME/Library/Application Support/GOmul/YOUR_AVD_NAME/bridge.json" \
     --game /path/to/your-game.zip \
     --data /path/to/your-data-folder \
     --phone-number YOUR_11_DIGIT_IDENTITY
   ```

   Select the flat data folder containing `savedata`, `locdata`, `ranker`, and the `it…`/`mk…` files. The tool maps all supplied files into that game's database and configures the identity for that game only. It refuses to replace an existing game's save directory. It does not change the phone's real number or SIM.
4. Open the imported game in GOmul and manually reach the working main menu. If authentication fails, stop: creating a checkpoint would preserve that failure, not repair it.
5. While this game is active at the desired menu, run:

   ```sh
   python3 tools/mac-checkpoints/setup.py pin \
     --config "$HOME/Library/Application Support/GOmul/YOUR_AVD_NAME/bridge.json" \
     --game /path/to/your-game.zip
   ```

The protected startup checkpoint is independent of ordinary Quick Save and cannot be overwritten by it. The launcher restores it when this game is selected. Use `clear` in place of `pin` to clear the ordinary Quick Load/Undo slots without removing startup. Do not publish the resulting snapshot: it contains installed games and guest memory.

No protected state is shipped, and this procedure does not promise compatibility with every dump/version. The tested identity/data combination reached the menu; further gameplay testing is needed.

The alpha.3 open-time auto-import change was rebuilt without running tests or gameplay checks, as requested.
