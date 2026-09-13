# 출처 및 제3자 라이선스 고지

GOmul은 **Inseok Lee(dlunch)와 기여자들의 [WIE](https://github.com/dlunch/wie)**를 기반으로 합니다. 시작 리비전은 `1ed87109`이며 원래 Git 이력과 [MIT 라이선스](LICENSE)를 유지합니다. GOmul의 추가 작업도 MIT로 배포합니다.

## 포함된 의존성과 자산

| 구성 요소 | 저자 / 출처 | 고지 위치 |
| --- | --- | --- |
| arm32_cpu | Sean Purcell | `vendor/arm32_cpu/LICENSE` (MIT) |
| RustJava JVM | Inseok Lee | `vendor/jvm/LICENSE` (MIT) |
| RustJava runtime | Inseok Lee | `vendor/rustjava-runtime/LICENSE` (MIT) |
| NeoDunggeunmo 글꼴 | 해당 프로젝트 저작권자 | `wie-web/public/licenses/NeoDunggeunmo-OFL.txt` (SIL OFL 1.1) |
| GeneralUser GS 사운드폰트 | 해당 자산 저작권자 | `wie-web/public/licenses/GeneralUser-GS-LICENSE.txt` (별도 배포 조건) |
| Gradle wrapper | Gradle 기여자 | 소스 헤더 및 [Apache-2.0](https://github.com/gradle/gradle/blob/master/LICENSE) |

추가 npm/Rust 고지는 `wie-web/public/licenses/`에 있습니다. Android 빌드 과정은 의존성 고지를 생성해 APK의 라이선스 화면에 포함합니다. GOmul의 MIT 라이선스가 의존성·글꼴·사운드폰트의 개별 라이선스를 대체하지 않습니다. 법적 원문은 번역으로 대체하지 않고 그대로 보존합니다.

## 비교에 참고한 프로젝트

[movingwoo/wfeature](https://github.com/movingwoo/wfeature), MIT, 검토 리비전 `a621a6a56dbc6e8f5da967d23218ec750d73001c`.

W-Feature의 변경 내역과 구조 문서는 비교할 영역을 찾는 데 참고했습니다. 저장소의 독립 구현 정책에 따라 해당 프로젝트의 코드나 알고리즘을 복사하지 않았습니다. 실제 구현 계약은 MIDP/JSR-135 등 공개 명세와 재현 가능한 테스트를 근거로 확인했습니다. 기반 코드인 WIE와 비교 참고 자료인 W-Feature를 구분합니다. 상세 내용은 [참고 범위](docs/references.ko.md)에 있습니다.

## 배포 범위

기존 Hello World ZIP 두 개와 ARM 합성 명령 바이너리는 소프트웨어 테스트 자료입니다. Gradle wrapper JAR는 빌드 도구입니다. 상용 게임, 게임 저장 번들, Action Hero 인증 데이터, 기기 체크포인트는 포함하지 않습니다.

[PROVENANCE.json](PROVENANCE.json)의 식별자는 GOmul 변경을 설명하기 위한 것이며 다른 저작자의 기여에 대한 소유권을 주장하지 않습니다. 기존 [출처 식별 설명](docs/provenance.md)도 참고하세요.
