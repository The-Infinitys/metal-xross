
# Metal Xross
![](./thumbnail.png)

**Metal Xross** is a high-performance, modern distortion plugin built with Rust using the `nih-plug` framework and `egui`. Designed for guitarists and sound designers who demand aggressive yet controllable saturation, it combines four distinct distortion algorithms with a precision 3-band parametric equalizer.

## ⚡ Features

  * **Quad-Style Distortion Engine**: Seamlessly morph between four handcrafted distortion models:
      * **Crunch**: Vintage tube-like saturation with dynamic response.
      * **Drive**: Balanced overdrive for versatile rock tones.
      * **Distortion**: High-density, saturated clipping for modern hard rock.
      * **Metal**: Ultra-high gain with razor-sharp edges and massive sustain.
  * **Pre-Emphasis Style Shaping**: Adjust `Style Low/Mid/High` to shape the frequency response *before* it hits the distortion stage, allowing for tight palm mutes or fuzzy, thick leads.
  * **Visual 3-Band Parametric EQ**: A post-distortion equalizer with real-time visual feedback. Fine-tune your final tone by carving out harsh frequencies or boosting the "body."
  * **Integrated Dynamics**:
      * Dual-stage **Noise Gate** (Pre/Post distortion) for tight, percussive riffing.
      * Input/Output **Limiters** to ensure your signal never clips your DAW's master bus.
  * **Cyberpunk UI**: A bespoke, hardware-inspired interface powered by `egui`, featuring smooth knob responses and a futuristic vector-based layout.

## 🛠 Signal Chain

The internal processing follows a professional studio rack logic:

1.  **Input Stage**: Gain adjustment and initial Limiting.
2.  **Pre-Gate**: Removes guitar pickup hum before amplification.
3.  **Xross Gain System**: The core distortion engine with 4 styles and 3 pre-emphasis filters.
4.  **Post-Gate**: Suppresses noise floor amplified by high-gain settings.
5.  **Equalizer**: 3-band parametric EQ (Low, Mid, High) for final coloring.
6.  **Output Stage**: Final volume leveling and safety limiting.

## 🚀 Getting Started

### Prerequisites

  * [Rust](https://www.rust-lang.org/) (Latest stable or nightly)
  * Compatible DAW (for VST3/CLAP/AU) or use the Standalone mode.

### Build and Run

This project uses `xtask` for streamlined development.

```bash
# Run the standalone version immediately
cargo run --release

# Bundle the plugin for use in your DAW (VST3, etc.)
cargo bundle --release
```

The bundled plugin will be located in the `target/` directory under the respective format folder.

## 🎛 Parameters

| Group | Parameter | Description |
| :--- | :--- | :--- |
| **General** | Gain | Overall saturation intensity. |
| **Style** | Kind | Morphs between Crunch, Drive, Distortion, and Metal. |
| **Style EQ** | Pre Low/Mid/High | Shapes the "bite" and "weight" of the distortion. |
| **EQ** | Freq / Q / Gain | Post-distortion tonal shaping for 3 bands. |
| **I/O** | Limit | Sets the ceiling for the internal limiters. |

## 📜 License

This project is licensed under the **MIT License** (or specify your preferred license).
