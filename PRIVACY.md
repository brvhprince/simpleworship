# SimpleWorship Privacy Policy

**Effective date:** 26 October 2026

SimpleWorship is free, open-source worship presentation software, made by
Prince Takyi Akomea and its contributors. It works offline and has no
accounts, no analytics and no advertising. **We do not collect your data:
nothing about you or your church is sent to us.** This page says exactly what
the app keeps, and which services it talks to and when, so you can check it
for yourself (the [source code](https://github.com/brvhprince/simpleworship)
is public).

## 1. What stays on your computer

- Your songs, Bibles, schedules, themes, presentations, pictures, videos,
  sound files, settings and the CCLI song usage log are stored on the
  computer SimpleWorship runs on. Nothing is uploaded anywhere.
- The app keeps logs on the computer, to help find problems. They are never
  sent automatically; **Help › Report a Problem** only opens GitHub in your
  browser, and you choose what to write there.
- There are no accounts, no sign-in, no analytics, no telemetry and no
  advertising.

## 2. Microphone (Listen)

- Only when you turn on **Listen** does SimpleWorship hear the input you
  choose (usually the mixer's feed), to find the scripture the preacher reads.
- Speech to text (Whisper) runs **on the computer**, in a helper program of
  SimpleWorship's own. Sound is never saved to disk and never sent over the
  internet or to any AI service.
- The speech model is downloaded once (see section 6).

## 3. Cameras

- SimpleWorship opens a camera or capture card only when you show it, in
  **Media › Cameras** or as a background.
- Camera pictures are shown on the screens of that computer only. They are
  deliberately left out of the stream output and NDI, are not shown on phones
  or on another computer that joins this one, and are never sent to the
  internet.

## 4. Your church's network

When you turn these on, SimpleWorship serves them to other devices on your
church's own network (Wi-Fi or cable), never over the internet:

- **Phone remote:** phones and tablets in their web browser. A phone must be
  paired with a code shown on the computer, which you approve, and each phone
  can only do what its role allows. Remove a phone in Settings › Phone Remote
  and it is disconnected at once.
- **Another SimpleWorship computer** that joins this one (as a second operator
  or a mirror) receives the service, its pictures and videos.
- **The stream output** for OBS, vMix or Streamlabs, and **NDI**, carry the
  slides shown, on the network.

The server uses port 7270 by default; you can change it in Settings.

## 5. The AI assistant (optional, off unless you turn it on)

SimpleWorship can use an AI service with **your church's own API key** for
Claude (Anthropic), ChatGPT (OpenAI) or Gemini (Google). It is off until you
turn it on in Settings › Assistant and give a key, which is kept in the
computer's keychain (macOS Keychain, Windows Credential Manager, or the
Secret Service on Linux), never in the library.

Nothing is sent until you click one of its buttons, and then only what that
button needs goes to the service you chose:

- **Ask AI:** the description you type (a song or a theme to find passages
  for). Your song library is never sent.
- **Tidy Lyrics** and **Translate:** the lyrics in front of you.
- **From Sermon Notes:** the notes you paste.

That text then goes to the service as part of your church's account with
it, and is handled under the terms for its API (not its consumer apps):
[Anthropic](https://www.anthropic.com/legal/commercial-terms) (see also its
[privacy centre](https://privacy.anthropic.com/)),
[OpenAI](https://openai.com/enterprise-privacy/) and
[Google](https://ai.google.dev/gemini-api/terms). Check the terms for your
plan: with some free plans the service may use what is sent to improve its
products; on Gemini's unpaid tier, for example, Google says people may read
it. Do not send anything through the assistant that you would not share with
that service.

## 6. Internet connections the app makes

SimpleWorship connects to the internet only for these. Each service sees the
computer's IP address, as with any download. Update checks and downloads send
no personal information; the assistant sends only what you choose to send it
(section 5).

- **Updates:** at start-up and once a day, if allowed in Settings, and when
  you click Check for Updates, the app asks GitHub
  (`github.com/brvhprince/simpleworship/releases`) whether there is a newer
  version, and downloads it in the background. It is only installed when you
  say so. Copies installed from the **Microsoft Store** never do this: the
  Store updates them.
- **Speech model for Listen:** downloaded once, when you choose to, from
  Hugging Face (`huggingface.co/ggerganov/whisper.cpp`).
- **More Bibles:** when you open **Get More Bibles**, the list of translations
  is fetched from eBible.org (and again when it is a week old), and a Bible
  you choose is downloaded from there. The Bibles included with SimpleWorship
  need no download.
- **The AI assistant**, only as described in section 5.
- **Links** you click (the website, the guide, Report a Problem) open in your
  browser.

## 7. The website

The website (`brvhprince.github.io/simpleworship`) is hosted by GitHub Pages.
It uses no cookies, analytics or advertising. To show its fonts it loads them
from Google Fonts, and to link to the newest download it asks GitHub's API
which version is newest, so those services see your IP address, as GitHub
does for hosting the page.

## 8. Microsoft Store

If you install SimpleWorship from the Microsoft Store, Microsoft's own
privacy policy covers the Store, your download and the Store's updates.
SimpleWorship itself behaves as described here.

## 9. Children

SimpleWorship collects no personal information from anyone, including
children.

## 10. Changes to this policy

If what SimpleWorship does with data changes, this page changes with it and
gets a new effective date. Each version is kept in the project's history on
GitHub.

## 11. Contact

Questions about this policy: open an issue at
[github.com/brvhprince/simpleworship/issues](https://github.com/brvhprince/simpleworship/issues).
The maintainer is Prince Takyi Akomea.
