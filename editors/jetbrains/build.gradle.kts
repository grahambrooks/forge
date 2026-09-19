import org.jetbrains.intellij.platform.gradle.IntelliJPlatformType
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlin)
    alias(libs.plugins.intellijPlatform)
}

group = providers.gradleProperty("pluginGroup").get()
version = providers.gradleProperty("pluginVersion").get()

kotlin {
    jvmToolchain(providers.gradleProperty("javaVersion").get().toInt())
    compilerOptions {
        jvmTarget = JvmTarget.JVM_21
        freeCompilerArgs.add("-Xjvm-default=all")
    }
}

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        intellijIdeaCommunity(providers.gradleProperty("platformVersion"))
        pluginVerifier()
        zipSigner()
    }

    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.platform.launcher)
    testRuntimeOnly(libs.junit4)
}

intellijPlatform {
    pluginConfiguration {
        name = providers.gradleProperty("pluginName")
        version = providers.gradleProperty("pluginVersion")

        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
            // No upper bound: the plugin uses long-stable platform APIs only.
            untilBuild = provider { null }
        }
    }

    signing {
        certificateChain = providers.environmentVariable("CERTIFICATE_CHAIN")
        privateKey = providers.environmentVariable("PRIVATE_KEY")
        password = providers.environmentVariable("PRIVATE_KEY_PASSWORD")
    }

    publishing {
        token = providers.environmentVariable("PUBLISH_TOKEN")
    }

    pluginVerification {
        ides {
            // recommended() stops at 2025.2, the last separate Community release, so the unified
            // IDEA distribution is listed explicitly. 2026.2 is where JCEF moved out of the core.
            recommended()
            create(IntelliJPlatformType.IntellijIdea, "2026.2.3")
        }
    }
}

// The plugin renders through the forge CLI, and the examples live in the crate.
val repoRoot: Directory = layout.projectDirectory.dir("../..")
val crateDir: Directory = repoRoot.dir("forge")

tasks.runIde {
    // Open the crate's examples so a launch lands on something to preview.
    args(crateDir.dir("examples").asFile.absolutePath)
    // -PopenFile=<path relative to the examples> also opens that file, e.g. -PopenFile=payments.forge
    providers.gradleProperty("openFile").orNull?.let {
        args(crateDir.dir("examples").file(it).asFile.absolutePath)
    }
}

// ./gradlew runRustRover -PrustRoverPath=<RustRover.app/Contents> runs the plugin in a local
// RustRover install, to check a platform newer than the one it is compiled against.
val runRustRover by intellijPlatformTesting.runIde.registering {
    providers.gradleProperty("rustRoverPath").orNull?.let { localPath = file(it) }
    task {
        args(crateDir.dir("examples").asFile.absolutePath)
        args(crateDir.dir("examples").file("payments.forge").asFile.absolutePath)
    }
}

tasks.test {
    useJUnitPlatform()
    // Tests that drive the real CLI use the debug build from the crate when it exists
    // (`make build`), and are skipped otherwise.
    systemProperty("forge.binary", crateDir.file("target/debug/forge").asFile.absolutePath)
    systemProperty("forge.examples", crateDir.dir("examples").asFile.absolutePath)
    testLogging {
        events("failed")
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}
