# Credits and third-party notices

GOmul is based on WIE by Inseok Lee (dlunch) and contributors, under MIT. The original notice remains in LICENSE. Upstream: https://github.com/dlunch/wie . GOmul's starting revision was 1ed87109; upstream history is retained.

Vendored modified dependencies retain their notices:

- arm32_cpu by Sean Purcell: vendor/arm32_cpu/LICENSE (MIT).
- RustJava JVM by Inseok Lee: vendor/jvm/LICENSE (MIT).
- NeoDunggeunmo font: wie-web/public/licenses/NeoDunggeunmo-OFL.txt (SIL OFL 1.1).
- GeneralUser GS soundfont: wie-web/public/licenses/GeneralUser-GS-LICENSE.txt (its own redistribution terms).
- Gradle wrapper: Apache-2.0; source scripts retain their notice. https://github.com/gradle/gradle/blob/master/LICENSE

Additional npm/Rust notices are in wie-web/public/licenses/. The native Android build generates its dependency notices into the APK's Licences page. Those dependency licences are not replaced by GOmul's MIT licence.

The two upstream Hello World ZIP fixtures and arm32_cpu synthetic instruction binaries are software test fixtures, not commercial games or game saves. They are retained for correctness testing. The Gradle wrapper JAR is build tooling. No Action Hero data or full-device checkpoint is included.

GOmul additions retain MIT licensing. The GOmul contribution notice and
[provenance identifiers](PROVENANCE.json) do not assert ownership of WIE or other
third-party contributions. See [provenance guidance](docs/provenance.md).
