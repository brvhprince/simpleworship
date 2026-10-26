# SimpleWorship Privacy Policy

**Effective Date:** October 26, 2026

SimpleWorship is free, open-source church presentation software developed by Prince Takyi Akomea. SimpleWorship is built offline-first and private by design. **We do not collect, store, sell, or transmit personal data.**

---

## 1. No Personal Data Collection

SimpleWorship does not collect personal identifiers or sensitive information:
- No user accounts, registration, or logins are required.
- No analytics, telemetry, or user behavior tracking scripts exist in the software.
- All presentation slides, song databases, scripture selections, schedules, themes, and configuration files are stored entirely on your local computer.

---

## 2. Hardware and Device Permissions

To deliver presentation and worship features, SimpleWorship may request hardware access:

### Microphone Access ("Listen" Feature)
- When you explicitly enable the **Listen** feature, SimpleWorship captures audio from your chosen input device to detect scripture references spoken during sermons.
- **100% Local Speech Recognition:** Audio transcription is executed entirely on your device using local Whisper AI models.
- Audio is **never recorded to disk, never streamed over the internet, and never shared with third parties or cloud AI services**.

### Camera & Capture Card Feeds
- SimpleWorship can access connected video cameras and capture cards under **Media › Cameras**.
- Video frames are rendered locally for preview and projector output, or streamed to local network switchers (via NDI).
- Video feeds are never sent to external servers or cloud services.

### Local Network Access (Phone Remote)
- The phone remote operates exclusively on your local Wi-Fi / Local Area Network (LAN) using a local web server running on port 7270.
- All communications are strictly peer-to-peer within your local network. No external servers or cloud relays are used.

---

## 3. External Network Connections

SimpleWorship makes external network calls only for user-initiated or necessary software functions:
- **Application Updates:** Periodically checks the GitHub Releases API (`https://api.github.com/repos/brvhprince/simpleworship/releases/latest`) to inform you when a new version is available. No personal information is sent.
- **Model Downloads:** Downloads Whisper speech recognition models directly from public model repositories when you choose to download them.
- **Bible Downloads:** Downloads public-domain USFM Bible translations directly upon request.

---

## 4. Open Source Transparency

SimpleWorship is free and open-source software licensed under the GNU General Public License v3.0 or later. The complete source code is publicly accessible for auditing at:
[https://github.com/brvhprince/simpleworship](https://github.com/brvhprince/simpleworship)

---

## 5. Contact Information

If you have questions or concerns about this Privacy Policy, please open an issue on GitHub:
- GitHub Issues: [https://github.com/brvhprince/simpleworship/issues](https://github.com/brvhprince/simpleworship/issues)
- Maintainer: Prince Takyi Akomea
