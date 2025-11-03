plugins {
    kotlin("jvm") version "1.9.21"
}

group = "io.github.chencmd"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation("io.ksmt:ksmt-core:0.6.4")
    implementation("io.ksmt:ksmt-yices:0.6.4")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.9.2")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    jvmToolchain(21)
}
