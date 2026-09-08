# Super Action Hero 3: local setup

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
