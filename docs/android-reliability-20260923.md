# Android 0.1.1 안정성 점검 — 2026-09-23

## 수정한 문제

1. **일시 정지 중 시간 누적**: 기존 APK는 3초 일시 정지 뒤 약 0.25초 실행한 검사에서 게스트 시계가 3,254 ms 전진했습니다. 일시 정지 및 체크포인트 작업 시간을 제외하도록 수정했습니다. 실행 중의 타이머 간격·취소 규칙·CPU 실행 정책·렌더링 제한은 변경하지 않았습니다.
2. **포커스를 잃은 가상 키**: 기존 APK에서 누른 키가 있는 상태로 대화상자를 열었을 때 해제되지 않는 회귀 테스트가 실패했습니다. 일반 Android와 Thor 모두 창 포커스가 사라지면 키를 해제합니다.
3. **Activity 교체 경합**: 네이티브 시작/종료 작업을 프로세스 내 한 작업 큐로 직렬화하고, 끝난 Activity에 비동기 결과를 전달하지 않습니다. 오래된 I/O·정리 작업이 새 게임 세션을 종료하거나 닫힌 화면에 다시 대화상자를 표시하는 경로를 막습니다.
4. **저장 레코드의 부분 덮어쓰기**: 기존 파일을 먼저 잘라 쓰는 대신 같은 디렉터리의 임시 파일에 완성한 뒤 이름을 바꿔 교체합니다. 쓰기 실패 시 이전 레코드가 남습니다. 미완성 임시 파일과 비레코드 파일은 레코드 목록에서 제외합니다. 여러 레코드 전체의 원자성이나 전원 차단 내구성을 보장한다는 뜻은 아닙니다.
5. **배포 및 안내**: 디버깅을 끈 서명 배포 빌드, 버전 코드 9 / 표시 버전 0.1.1을 추가했습니다. 기존 인증서로 업데이트 호환성을 유지합니다. 빠른 저장의 빈 슬롯·다른 빌드·재구성 실패 안내를 한국어로 제공합니다.

## 실제 검사

환경은 Mac에서 실행한 Android API 36 ARM64 AVD입니다. 먼저 원래 AVD를 읽기 전용으로 실행한 임시 환경에서 검증했습니다. 실제 Fold/Thor 기기의 검증이나 전체 게임 플레이 완료를 의미하지 않습니다.

| 검사 | 결과 |
| --- | --- |
| 전체 Rust 작업 공간 | 578개 통과, 실패 0개, 개인 SKT 파일 필요 테스트 1개 제외 |
| 마지막 변경 후 Android Rust 재검사 | 31개 통과; 테스트 빌드에서도 슬롯 검사를 생략하지 않도록 수정 |
| 실제 APK의 Thumb inline/table 구성 | CPU 테스트 88개 통과 |
| Java 가져오기·저장 초기화·컨트롤 검사 | 3개 실행기 통과 |
| Android lint release | 오류 0개, 경고 8개(기존 스타일·접근성·빌드 도구 안내 등) |
| 포맷·workspace clippy | 통과; 기존 경고 있음 |
| 실제 네이티브 빠른 저장 | 3개 슬롯, 재시작 후 불러오기, 다른 빌드 거부, 재구성 실패 롤백, 불러오기 취소 복구, 다른 게임 분리 통과 |
| 실제 Android 프로세스 강제 종료 | 실행 중 프로세스를 force-stop한 뒤 새 프로세스에서 세 슬롯 모두 재구성 통과 |
| 저장 중단 복구 | 불러오기 중단 상태를 파일시스템으로 재현한 Rust 테스트에서 다음 실행 롤백 및 커밋 후 유지 통과 |
| 실제 Activity 수명 주기 | 가상 키 누름·포커스 상실, Home, 복귀 시 시계, 회전, I/O 대기 중 Activity 교체, 화면 잠금·해제 통과 |
| 실제 오디오 경로 | 무음 PCM의 재생·일시 정지·복귀, 정지 중 속도 변경, 종료 후 플레이어/파일/스레드 정리 통과. 청감 품질 인증은 아님 |
| APK 업데이트 | 기존 버전 7→8→9 설치 성공. 기존 저장·체크포인트·설정 파일 2,374개의 SHA-256 모두 동일 |
| 반복 세션 | 레이카르나 시작/종료 6회 정상. 첫 회 이후 RSS 약 395.5–396.1 MiB, 메모리 맵 2,940개로 유지. 장시간 누수 부재의 증명은 아님 |
| 공개 파일·이력 검사 | 12,629개 파일/이력 블롭에서 정해진 금지 패턴 0건 |

기존 APK는 새 검사에서 가상 키 포커스 해제 및 일시 정지 시계 조건에 실패했고, 수정 APK는 두 조건 모두 통과했습니다. 테스트만 통과시키기 위한 게임별 분기나 FPS 조정은 추가하지 않았습니다.

## 대표 게임 점검의 범위

미니게임천국1·2(KTF), 놈3(KTF), 이노티아1(KTF), 레이카르나(LGT), 슈퍼액션히어로3(LGT)는 제한된 부팅/입력 시퀀스에서 치명적 오류 없이 실행되었습니다. 대부분 기존 저장의 **복사본**을 사용했으며, 저장이 없는 미니게임천국2는 빈 저장에서 다시 검사했습니다. 첫 미니게임천국2 실행은 테스트 준비 경로가 없어 실패한 것이므로 성공 횟수에 포함하지 않았습니다.

추가로 검사한 **놈3 LGT 파일은 기존/수정 APK 모두 같은 퍼블리셔 화면에 머물렀습니다**. 최종 화면 해시도 같았습니다. 이번 변경으로 생긴 회귀는 아니지만 이 파일의 플레이 가능 여부를 통과로 표시하지 않습니다. KTF 파일과 혼동하지 마세요. 별도 호환성 조사가 필요한 항목입니다.

부팅 통과는 엔딩·모든 장면·모든 저장 상태의 보증이 아닙니다. 기존 호환성 분류를 일괄 승격하지 않습니다.

## 재실행 방법

게임·개인 저장·기기 스냅샷·오류 ZIP은 저장소에 넣지 않습니다. 본인 게임을 넣은 별도 AVD에서 배포 APK와 같은 인증서로 테스트 APK를 빌드·설치한 후 다음 instrumentation 모드를 사용합니다.

```sh
./wie-android/android/gradlew -p wie-android/android :app:assembleDebugAndroidTest
adb install -r wie-android/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell am instrument -w -r -e mode checkpoint-reliability -e game /path/on/device/game.zip local.wie.nativeapp.test/local.wie.nativeapp.RescueInstrumentation
adb shell am instrument -w -r -e mode android-lifecycle -e game /path/on/device/game.zip local.wie.nativeapp.test/local.wie.nativeapp.RescueInstrumentation
adb shell am instrument -w -r -e mode audio-timing local.wie.nativeapp.test/local.wie.nativeapp.RescueInstrumentation
```

`checkpoint-process-seed`는 별도 캐시 저장에 세 슬롯을 만들고 `cache/checkpoint-process-test/ready` 표시 후 강제 종료를 기다립니다. 해당 프로세스를 종료한 다음 같은 게임으로 `checkpoint-process-reopen`을 실행하면 새 프로세스에서 검증 후 테스트 데이터를 지웁니다. 테스트는 고유한 테스트용 저장 식별자를 쓰며 이미 존재하면 덮어쓰지 않고 거부합니다.

일반 저장은 업데이트 시 유지됩니다. 빠른 저장은 여전히 생성한 빌드에 종속되므로 업데이트 전 게임 안에서도 저장하는 것이 좋습니다. 기존 개발 인증서를 유지한 APK이며 새로운 스토어용 키로 전환한 것은 아닙니다. 배포 절차는 [빌드 안내](build.md), 사용자 제한은 [빠른 저장 안내](android-checkpoints.md)를 참고하세요.
