# MetalXross

**MetalXross** is a high-gain distortion plugin built with Rust and the `truce` framework. It features a unique "Xross Gain" engine that allows users to morph seamlessly between four distinct distortion algorithms, providing everything from vintage crunch to modern extreme metal tones.

## 🚀 Key Features

* **Xross Gain Morphing**: Seamlessly interpolate between `Crunch`, `Drive`, `Distortion`, and `Metal` engines with a single parameter.
* **Intelligent Noise Gate**: Features a frequency-conscious multi-band detection sidechain. It preserves picking nuances while aggressively cutting hum and hiss.
* **Post-Gate Smoothing**: An adaptive Low-Pass Filter (LPF) kicks in as the gate closes, preventing "digital silence" awkwardness and providing a natural decay.
* **3-Band Active EQ**: Precision-tuned Biquad filters optimized for the "sweet spots" of electric guitar frequencies.
* **Ultra-Low Latency**: Optimized for real-time performance. Stable even at a buffer size of **64 samples**, making it ideal for live tracking.

## 🎛 Parameters

| Parameter | Description |
| :--- | :--- |
| **Style** | Selects and morphs between distortion types (0.0: Crunch ~ 3.0: Metal) |
| **Gain** | Controls input drive and the depth of saturation |
| **Gate Threshold** | Sets the noise gate floor (dB) |
| **Gate Release** | Adjusts how quickly the gate closes (ms) |
| **EQ (Low/Mid/High)** | Active frequency shaping for each band |
| **Output Limit** | Final output ceiling/limiter (dB) |

## 🛠 Setup & Build

To build this plugin, you will need the Rust toolchain installed.

1.  **Clone the Repository**
    ```bash
    git clone https://github.com/The-Infinitys/metal-xross.git
    cd metal-xross
    ```

2.  **Install with Truce**
    ```bash
    cargo truce install
    ```

3.  **Formats**
    Depending on your `truce` configuration, the VST3 and AU (Audio Unit) binaries will be generated in the `target/release` directory.

## 📝 Technical Specifications

* **Framework**: `truce` (Plugin Framework)
* **DSP Engine**: 
    * 32-bit floating-point internal processing
    * Direct Form II Transposed Biquad Filters
    * Hysteresis-based Gate Logic
* **Architecture**: Non-interleaved Audio Buffer (Zero-copy memory management)

## ⚠️ Performance Notes (Linux/ALSA)

When running at extremely low buffer sizes (64 samples or lower), you may encounter `underrun occurred` errors depending on your system configuration. To mitigate this:
* Set your CPU frequency governor to `performance`.
* Ensure your user is part of the `audio` group with real-time priorities.
* Increase the buffer size to 128 samples if audio crackling persists.

---

## License

[MIT License](LICENSE)

---

### Credit
Developed with ❤️ using **Rust** for the next generation of digital guitar tones.

## 🎸 Show me your sound!
If you use **MetalXross** in your tracks, videos, or projects, I would love to hear it! 
Feel free to open an Issue, tag me on social media, or send a link. Seeing how this 
distortion engine shapes your music is my greatest motivation.
