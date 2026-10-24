# Changelog

What changed in each release of SimpleWorship. The newest release is at
the top. Each release's notes are also shown in the app when it offers the
update.

## 0.5.1 (24 October 2026)

Camera listing scrolling and Listen audio stability fixes.

- **Camera listing scrolling:** Added scrolling to the Find Cameras listing
  cards in the Media tab so "Add to Schedule" and "Show in Preview" buttons
  remain accessible on smaller screens and compact panels.
- **Listen stability & responsiveness:** Decoupled speech transcription from
  real-time audio capture into separate worker threads. Audio level metering
  continues smoothly without freezing while Whisper transcribes, and audio
  buffers no longer accumulate.
- **CPU inference throttling:** Limited CPU Whisper inference to 2 worker
  threads on Intel Mac and non-accelerated systems, preventing thread
  starvation of CoreAudio and WebKit.
- **Realtime safety:** Non-blocking audio stream buffer delivery, sample
  sanitization, and rate-limited glitch warnings to prevent CoreAudio thread
  priority inversion.
- **Defensive scripture slicing:** Guarded UTF-8 string slicing in the spoken
  scripture spotter to prevent boundary panics.

## 0.5.0 (24 October 2026)

Testing feedback fixes, category themes, scripture auto-flow, and UI refinements.

- **Theme categories:** Default themes can now be chosen separately for
  Songs, Scripture, Presentations, and Quick Text (Settings › Themes). The
  Theme Designer adapts to the selected category, hiding song-specific
  elements for non-song themes.
- **Color presets:** A quick color palette picker in the Theme Designer
  makes selecting high-contrast and worship-friendly colors easy.
- **Media fit modes:** Choose whether image and video backgrounds contain,
  cover, or stretch to fill the screen, previewed immediately on thumbnail
  cards.
- **Scripture reference preservation & auto-flow:** Entering scripture
  verse ranges (e.g. Esther 8:7-10) now preserves the entire range in the
  slide caption across preview and live displays. Multi-verse passages are
  automatically flowed into readable slides dynamically sized to the
  chosen font. The Scripture tab preserves search state and translations
  when switching tabs.
- **Media search:** Fast instant search bar added to the Videos and Images
  tabs in the media library.
- **Quick Text button:** Dedicated Quick Text launcher icon in the operator
  navigation bar for rapidly sending announcements or urgent notes.
- **Configurable preview advancement:** An option in Settings allows
  toggling whether triggering a live slide automatically advances the
  preview to the next slide.
- **Schedule layout switcher:** Switch between list, grid, and compact view
  modes in the schedule panel.
- **Presentation editor refinements:** Added clear placeholders and bold
  text toggle in the presentation formatting toolbar.
- **Audio stability & mirror synchronization:** Audio overrun and underrun
  handling with automatic protection against buffer accumulation; keep
  joined computers' lists and schedule names in sync.

## 0.4.0 (20 October 2026)

Listen for scripture, and an assistant.

- **Listen:** turn on Listen in the toolbar and choose the microphone (the
  mixer's feed). SimpleWorship hears the preacher, and every scripture they
  read ("John chapter three, verse sixteen", "Romans 8:28 to 30", "verse
  seventeen") appears at the top of the Scripture tab. Click it, or press
  **H** for the newest, to show it in the preview; double-click it to send
  it live. Nothing goes live by itself. Speech to text runs on the
  computer, so it works offline and no sound leaves the church; its speech
  model is downloaded once. It understands English; in a Twi sermon it
  catches references said in English.
- **Assistant (optional):** with the church's own API key for Claude,
  ChatGPT or Gemini (Settings › Assistant), an AI can help:
  - **Ask AI** in the Songs tab finds songs from a description ("the song
    about the blood that never loses its power"), and in the Scripture tab
    finds passages on a theme;
  - **Tidy Lyrics** sets out pasted lyrics in verses and choruses, without
    chords;
  - **Translate** drafts a song in Twi, French or English, as a new song;
  - **From Sermon Notes** in the Text tab turns the preacher's outline into
    slides and scripture for the schedule.

  Nothing is sent until you click one of its buttons, and your library is
  never sent. The key stays in the computer's keychain.
- **Scripture:** clicking a verse shows it in the preview, and
  double-clicking sends it live.
- **Alerts** can have their own bar and text colours, and each alert
  template keeps its own.
- **Updates:** Check for Updates shows each step (checking, downloading,
  then Install), the toolbar shows a download's progress, and the update
  window shows what's new with proper headings and points.
- Fixes: the About window's layout, the slide pictures in the song editor,
  and the Scripture tab's reference box lining up with the rest.

## 0.3.0 (20 October 2026)

More than one computer.

- **Join another computer:** a second SimpleWorship computer can join the
  one running the service, from Remote › Join Another Computer. Pick it
  from the list, type the code shown in its Connect a Phone window, and
  its operator allows it:
  - as a **second operator**, with its own preview, for example scripture
    at the pulpit desk while songs run from the booth;
  - or as a **mirror**, which follows the service with its own screens
    or NDI, for an overflow room or the stream booth.

  Pictures and videos come from the main computer, the clocks are kept in
  step, and the sound plays on the main computer. Both computers need the
  same version of SimpleWorship.
- **Take over:** if the main computer stops answering, a joined computer
  offers to carry on the service, from the slide that was live, with the
  same schedule, screens, timer and music.
- **Each phone has its own preview,** so lining something up on a phone
  never disturbs the operator's preview.
- **What's new in each update** now shows in the update window.

## 0.2.0 (20 October 2026)

Phones, livestreams and cameras.

- **Phone remote:** control SimpleWorship from phones and tablets on the
  church Wi-Fi, in the browser (nothing to install). In the toolbar, click
  Remote › Connect a Phone and scan the QR code, then choose what the phone
  may do: full control, presenter (next, previous, Black), worship leader
  (the songs in the schedule), alerts (for the nursery and ushers) or a
  stage display on a tablet. Remove a phone in Settings and it is
  disconnected at once.
- **Livestream look:** the same slides in a look of their own for the
  stream, like EasyWorship's Alternate Output. By default, songs and
  scripture appear as a lower third over the camera. Black, Logo and Clear
  Text take the words off the stream without covering the camera. Choose
  the look for each kind of content in Settings › Stream.
- **OBS, vMix and Streamlabs:** copy the browser-source link from Stream in
  the toolbar; the background is see-through.
- **NDI:** send the stream look (and, if wanted, what the projector shows)
  to vMix, OBS, a TriCaster or any NDI receiver on the network, with
  transparency. Needs the free NDI Tools on the computer.
- **Stream window:** the stream look on a screen, for a capture card or a
  video switcher (with a green background for chroma key).
- **Cameras:** cameras and capture cards appear in Media › Cameras. Show
  one full screen, or use one as a background behind the words.
- **Transitions for each kind of content** (songs, scripture,
  presentations, everything else), remembered between services, with "Cut
  for everything".
- **A tidier toolbar** that fits on one line on any screen: Screens (for
  the projector, stage display and stream window), Transitions, Stream and
  Remote on the left; Black, Logo, Clear Text, Clear Background, Alert and
  Stage on the right.
- **Going live shows the words:** sending something live takes off Logo and
  Clear Text (Black stays on, as the safety net).
- **Fonts on Windows:** every installed font now appears in the font list,
  including those installed outside the Windows Fonts folder.
- **Twi** translation of the whole app.
- **Fixed:** importing EasyWorship media finds the Resources folder in more
  places and says where it looked.

## 0.1.0 (19 October 2026)

The first release: everything a Sunday service needs, for churches moving
from EasyWorship.

- **Live:** operator window with schedule, preview and live; projector and
  stage outputs on any monitor (opening on the projector by itself);
  Black, Logo, Clear Text, Clear Background; cut and fade transitions.
- **Songs:** library with instant search, editor with sections and
  arrangement, CCLI numbers and licence in the footer, usage report.
- **Scripture:** type references the way you say them (`jn 3:16-18`,
  `isa 29:12-13, 17-21`); search; two translations side by side; KJV and
  WEB included; import more, including EasyWorship `.ewb` Bibles.
- **Schedules:** drag to build, save as `.sws` (with themes and media, so
  they open on another computer), double-click a `.sws` to open it, crash
  recovery.
- **Themes:** designer with backgrounds (colour, gradient, picture, video,
  with opacity), text styling, a draggable text box, scripture reference
  and footer boxes, verse number styles.
- **Media:** pictures, videos and background music; video backgrounds that
  keep playing smoothly while the words change.
- **Presentations:** a canvas slide editor (text boxes with styled words,
  shapes, pictures, apply to all slides), PDF and PowerPoint import,
  slideshows, auto-advance.
- **Extras:** alerts and nursery messages, countdowns, stage timer and
  messages.
- **Moving from EasyWorship:** imports songs, Bibles, pictures, videos and
  sound; also OpenLP, ProPresenter 6, OpenLyrics, ChordPro and CCLI
  SongSelect.
- **Updates:** SimpleWorship looks for new versions and installs them only
  when you click, never during a service.
