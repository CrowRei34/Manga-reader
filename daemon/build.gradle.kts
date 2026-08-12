plugins {
    kotlin("jvm") version "2.2.10"
    kotlin("plugin.serialization") version "2.2.10"
    application
    id("com.gradleup.shadow") version "8.3.8"
}

group = "io.github.bakeneko"
version = "1.0.0"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

kotlin {
    jvmToolchain(21)
}

application {
    mainClass.set("io.github.bakeneko.daemon.MainKt")
}

// Fat JAR autocontenido con todas las deps (parsers, okhttp, nashorn).
tasks.shadowJar {
    archiveBaseName.set("bakeneko-daemon")
    archiveClassifier.set("")
    archiveVersion.set("")
    mergeServiceFiles()
}

repositories {
    google()
    mavenCentral()
    maven { url = uri("https://jitpack.io") }
}

dependencies {
    implementation("com.github.AppFuton:futon-parsers:f287c414a6")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.openjdk.nashorn:nashorn-core:15.4")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
}

tasks.test {
    useJUnitPlatform()
}