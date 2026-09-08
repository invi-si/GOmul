// This is applied only by the opt-in CPU diagnostic build preparation script.
// Keep the generated JNI namespace intact while isolating Android app storage.
if (providers.gradleProperty("wieDiagnostic").orNull == "true") {
    extensions.configure<com.android.build.api.dsl.ApplicationExtension>("android") {
        buildTypes.getByName("debug") {
            applicationIdSuffix = ".profile"
            resValue("string", "app_name", "WIE CPU Diagnostics")
            resValue("string", "main_activity_title", "WIE CPU Diagnostics")
        }
    }
}
