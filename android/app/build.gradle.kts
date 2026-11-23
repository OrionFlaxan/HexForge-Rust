plugins {
    id("com.android.application")
}

android {
    namespace = "com.orionflaxan.hexforge"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.orionflaxan.hexforge"
        minSdk = 21
        targetSdk = 36
        versionCode = 4
        versionName = "0.4.0"
        base.archivesName.set("HexForge-Rust")
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            isMinifyEnabled = false
            isDebuggable = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }
}

val buildRustAndroid = tasks.register<Exec>("buildRustAndroid") {
    group = "build"
    description = "Cross-compile libhexforge_app.so for all 4 Android ABIs (API 21+)"
    val skipRust = project.findProperty("skipRustBuild")?.toString()?.equals("true", ignoreCase = true) == true
    val script = rootProject.projectDir.parentFile.resolve("scripts/build-android.sh")
    onlyIf { !skipRust && script.exists() }
    workingDir = rootProject.projectDir.parentFile
    commandLine("bash", script.absolutePath)
}

tasks.named("preBuild") {
    dependsOn(buildRustAndroid)
}
