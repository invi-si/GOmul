# GOmul 직접 빌드

저장소를 복제한 뒤 루트 디렉터리에서 실행합니다. Rust stable(2024 edition 지원), Python 3, 웹 빌드에는 Node.js 24 이상이 필요합니다. 개인 경로·키·게임·빌드 산출물은 Git에 추가하지 않습니다.

## Android ARM64 APK

JDK 21, Android SDK 플랫폼 36, 빌드 도구, NDK 29를 설치하고 각 도구의 라이선스에 동의합니다. `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`을 본인 설치 위치로 설정하고 Java와 Android platform-tools를 PATH에 추가합니다.

```sh
rustup target add aarch64-linux-android
./wie-android/build.sh
```

산출물: `wie-android/android/app/build/outputs/apk/debug/app-debug.apk`

개발용 서명 APK입니다. 스토어용 정식 서명 배포판이 아닙니다. 게임 파일은 포함하지 않습니다. 기존 앱 위에 설치하려면 서명 키가 일치해야 하며, 직접 빌드한 APK는 공개 APK와 서명이 다를 수 있습니다. 업데이트 목적으로 앱을 삭제하면 저장도 지워질 수 있습니다.

```sh
adb install -r wie-android/android/app/build/outputs/apk/debug/app-debug.apk
```

AYN Thor 옵션은 같은 빌드 환경에서 `GOMUL_THOR=1 ./wie-android/build.sh`로 실행합니다. 상단 화면과 물리 컨트롤 중심의 별도 앱 ID를 사용합니다. [세부 기록](ayn-thor.md)을 참고하세요.

네이티브 빌드는 검증된 Thumb inline/table 옵션을 사용하며 지원하지 않는 실행 경로는 인터프리터로 돌아갑니다. 새로운 CPU 실험을 임의로 활성화하지 마세요.

## 브라우저 WebAssembly

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm ci
npm run build:prod
python3 -m http.server 8791 --bind 127.0.0.1 --directory wie-web/dist
```

`http://localhost:8791/index.html`에서 본인의 게임 파일을 가져옵니다. 산출물은 `wie-web/dist/`이며 에뮬레이터는 브라우저의 Web Worker에서 실행됩니다. 서버는 정적 파일만 제공해도 됩니다. 공개 호스팅은 HTTPS와 올바른 `.wasm` MIME 타입이 필요합니다.

로컬 게임 카탈로그를 이용하려면 정적 서버 대신 다음 명령을 사용합니다.

```sh
python3 tools/web-catalog/local_wasm.py --games /path/to/your/games --port 8791
```

`/path/to/your/games`는 본인이 준비한 디렉터리로 바꿉니다. 로컬 카탈로그 도구와 단순 파일 가져오기 화면의 기능은 다릅니다. 카탈로그를 정적 배포하려면 카탈로그 JSON·파일 경로도 별도로 제공해야 합니다. 상용 게임·표지의 공개 배포 권한은 이 소스 라이선스에 포함되지 않습니다.

과거 `npm run start:catalog`는 Python/네이티브 작업자가 게임을 실행하는 별도 실험입니다. 순수 WASM 배포와 혼동하지 마세요. 브라우저의 일반 저장은 IndexedDB에 보관되며 Android의 빠른 저장·구조 ZIP과 호환되는 실행 상태 형식이 아닙니다.

## 데스크톱 소스

CLI는 `cargo build --release`로 빌드합니다. Tauri를 사용하려면 해당 운영체제의 Tauri 빌드 의존성과 `tauri-cli`를 설치한 뒤 `wie-app`에서 `cargo tauri build`를 실행합니다. Windows 배포판은 Windows에서 별도 검증하기 전까지 검증 완료로 표시하지 않습니다.

## 선택 사항: macOS AVD 체크포인트 도우미

별도 Android Virtual Device에 APK를 설치하고 한 번 실행한 뒤 다음을 사용합니다.

```sh
python3 tools/mac-checkpoints/install-bridge.py --avd YOUR_AVD_NAME
```

이 도구는 로컬 토큰과 macOS LaunchAgent를 생성하며 선택한 AVD 한 개를 사용합니다. 전체 AVD 스냅샷 기반이라 OS·앱 업데이트에 영향을 받습니다. 물리 Android의 재실행 체크포인트와 다른 구현입니다. 소중한 저장이 있는 환경 대신 별도 테스트 AVD에서 사용하세요.

중지:

```sh
launchctl bootout "gui/$(id -u)/io.github.invi-si.gomul.checkpoints"
```

## 테스트와 공개 점검

```sh
cargo fmt --all --check
cargo clippy --workspace
cargo test --workspace --locked
cargo test -p wie-core-arm --features experimental-thumb-inline,experimental-thumb-table --locked
node --test wie-web/tests/*.test.mjs
node node_modules/typescript/bin/tsc --noEmit -p wie-web/tsconfig.json
python3 -m unittest discover -s tools/mac-checkpoints -v
python3 -m unittest discover -s tools/compatibility -v
python3 -m unittest discover -s tools/web-catalog -v
python3 scripts/release-audit.py
```

의존성 변경 후 `python3 scripts/generate-rust-notices.py`와 `python3 wie-android/generate-notices.py`로 고지를 갱신합니다. 공개 준비 시 실제 실행 결과와 환경 제한은 [검증 기록](public-update-20260914.md)에 남깁니다.
