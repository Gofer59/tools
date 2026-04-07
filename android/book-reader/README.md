# book-reader

Android app for reading physical books aloud. Point your phone's camera at a page, capture a frame, tap the text blocks you want to hear, and the app reads them out loud using text-to-speech. Supports French and English with a one-tap language toggle.

All processing happens on-device -- no internet connection required. OCR uses ML Kit's bundled Latin model, and TTS uses Android's built-in speech engine.

## Platform

Android 8.0+ (API 26+), built on Linux or macOS

## Prerequisites

To build from source:

- **Android Studio** (Hedgehog 2023.1 or later) or the Android SDK command-line tools
- **JDK 17+**

To install a pre-built APK:

- An Android phone running Android 8.0 or later

## Build

### With Android Studio

1. **File > Open** and select the `book-reader/` directory
2. Wait for Gradle sync to finish
3. Click **Build > Build Bundle(s) / APK(s) > Build APK(s)**

### From the command line

```bash
# Set these to match your system
export ANDROID_HOME=$HOME/Android/Sdk
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk

cd book-reader
./gradlew assembleDebug
```

The APK is generated at:

```
app/build/outputs/apk/debug/app-debug.apk
```

## Install

### Via USB (with ADB)

1. Enable Developer Options on your phone: **Settings > About phone > tap "Build number" 7 times**
2. Enable **USB debugging** in **Settings > Developer options**
3. Connect the phone via USB and run:

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

### Direct APK transfer

1. Copy `app-debug.apk` to your phone (USB file transfer, email, cloud storage, Bluetooth, etc.)
2. On the phone, open the file and allow installation from unknown sources when prompted
3. Tap **Install**
4. Launch **BookReader** from the app drawer

## Usage

### Basic workflow

1. Grant camera permission when prompted on first launch
2. Choose the language with the **FR/EN** button in the top-right corner
3. Point the camera at a book page
4. Tap **Capturer** (Capture) to freeze the frame
5. Detected text blocks appear highlighted in blue
6. Tap the blocks you want to read -- they turn orange when selected
7. Tap **Lire la selection** (Read selected) -- the text is read aloud
8. Tap **Effacer** (Clear) to return to the live camera

### Language toggle

The **FR/EN** button in the top-right corner switches between French and English for both OCR recognition and TTS voice. The default language is French.

### TTS voice setup

The app uses Android's built-in text-to-speech engine. If the voice for the selected language is not installed, a banner appears with an **Install** button that opens your device's TTS settings.

To install voices manually:

#### Samsung phones (Galaxy S, A, etc.)

1. **Settings > Accessibility > Text-to-speech** (or Settings > General management > Text-to-speech)
2. Check the selected engine (Samsung TTS or Google TTS)
3. Tap the **gear** icon next to the engine
4. **Language** > download **French (France)** and/or **English (United States)**

> **Samsung tip:** if voice quality is poor with Samsung TTS, install **Google TTS** from the Play Store and select it as the default engine.

#### Google Pixel phones

1. **Settings > System > Languages & input > Text-to-speech**
2. Tap the **gear** icon next to Google Speech Services
3. **Install voice data** > download **French** and **English**

#### Xiaomi / Redmi / POCO phones

1. **Settings > Additional settings > Accessibility > Text-to-speech**
2. Tap the **gear** icon next to the engine (usually Google TTS)
3. **Install voice data** > download **French** and **English**

> If Google TTS does not appear, install it from the Play Store: search "Google Text-to-Speech".

#### Universal method (all phones)

1. Open the **Play Store**
2. Search for **"Google Text-to-Speech"**
3. Install or update
4. Go back to **Settings** and search for **"Text-to-speech"** in the search bar
5. Select **Google** as the engine > **gear** > download voices

#### Verify the voice works

In the text-to-speech settings, there is a **"Listen to an example"** (or "Play") button. Tap it to verify the voice works correctly.

## Architecture

```
app/src/main/java/com/example/bookreader/
  MainActivity.kt           Activity: camera setup, button handlers, state observation
  CameraViewModel.kt        ViewModel: state machine (LivePreview -> Processing -> Frozen)
  OcrProcessor.kt           On-device OCR via ML Kit Text Recognition (bundled Latin model)
  TtsManager.kt             TTS wrapper with StateFlow, locale switching (FR/EN)
  HighlightOverlayView.kt   Custom View: bounding-box overlay, tap-to-select interaction
  model/
    CameraState.kt          Sealed interface: LivePreview | Processing | Frozen
    TtsState.kt             Sealed interface: Initializing | Ready | Speaking | MissingVoice | Error
    TextRegion.kt           Data class: detected text block with bounding box and selection state

app/src/main/res/
  layout/activity_main.xml  ConstraintLayout: camera preview, frozen image, overlay, buttons
  values/strings.xml        UI strings (French by default)
```

### Pipeline

```
Camera (CameraX) --> Capture frame --> Rotate to upright (Matrix.postRotate)
  --> ML Kit Text Recognition (on-device, bundled model)
  --> List<TextRegion> sorted in reading order (top-to-bottom, left-to-right)
  --> Display frozen frame + bounding box overlay
  --> User taps to select regions
  --> Concatenate selected text --> Android TTS engine --> Audio output
```

### Key dependencies

| Library | Purpose |
|---------|---------|
| CameraX 1.3 | Camera preview and frame capture |
| ML Kit Text Recognition 16.0 | On-device OCR (bundled Latin model, no network required) |
| Android TextToSpeech | Built-in speech synthesis |
| Material 3 | UI components and theming |

## Known Limitations

- **Portrait orientation only.** The app is locked to portrait mode for simpler coordinate mapping between the camera image and the overlay.
- **Latin script only.** ML Kit's bundled Latin model handles French, English, and other Latin-alphabet languages. It does not support CJK, Arabic, Devanagari, etc.
- **Lighting matters.** OCR accuracy drops with poor lighting, glare, or skewed pages. Hold the phone parallel to the page for best results.
- **No continuous scanning.** Each capture is a single frame -- there is no auto-detect or continuous reading mode.
- **TTS quality depends on the device.** Some manufacturers ship lower-quality TTS engines. Installing Google TTS from the Play Store generally gives the best results.
- **Minimum Android 8.0** (API 26) required.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Unknown sources" not found | Search for "install unknown apps" in settings |
| App won't install | Verify Android 8.0+ is installed (Settings > About phone) |
| OCR detects nothing | Light the page well, move the camera closer, avoid glare |
| Voice speaks the wrong language | Check the FR/EN button in the top-right corner and the installed TTS voice |
| No sound | Check media volume (not ringtone volume) |
| "Voice not available" | Follow the "TTS voice setup" section above |
| Crash on launch | Uninstall and reinstall the APK |

## License

MIT -- see [LICENSE](../../LICENSE)
