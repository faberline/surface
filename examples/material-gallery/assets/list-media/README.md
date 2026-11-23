# Gallery list media

The Gallery embeds two locally downsampled NASA images. The files are straight-alpha sRGB RGBA8 in top-to-bottom row order. They contain no image container metadata. The Gallery requires no network connection to display them.

| Fixture | Original and credit | Embedded size |
| --- | --- | --- |
| `blue-marble-160x160.rgba` | Apollo 17 astronaut photograph AS17-148-22727, NASA Johnson Space Center Gateway to Astronaut Photography of Earth. [Photo record](https://images.nasa.gov/details/as17-148-22727) · [NASA JPEG](https://images-assets.nasa.gov/image/as17-148-22727/as17-148-22727~small.jpg) | 160 × 160, 102,400 bytes |
| `new-zealand-poster-216x144.rgba` | ISS042-E-170757, the original poster of *ISS over New Zealand*. Earth Science and Remote Sensing Unit, NASA Johnson Space Center. [Video and frame range](https://eol.jsc.nasa.gov/BeyondThePhotography/CrewEarthObservationsVideos/SouthPacific.htm#newZealand_iss_20150124) · [NASA poster JPEG](https://eol.jsc.nasa.gov/BeyondThePhotography/CrewEarthObservationsVideos/videos/newZealand_iss_20150124/ISS042-E-170757.jpg) | 216 × 144, 124,416 bytes |

The Earth Science and Remote Sensing Unit makes astronaut photographs and time-lapse imagery available for reuse. The Gallery uses them to illustrate photo and video-poster layouts. [NASA JSC reuse guidance](https://eol.jsc.nasa.gov/faq/) · [NASA media usage guidelines](https://www.nasa.gov/nasa-brand-center/images-and-media/)

The embedded files use Pillow 12.3.0. Open the original JPEG, apply its EXIF orientation, convert to RGB, resize with Lanczos, convert to RGBA, and write `tobytes()`.

| File | Original JPEG SHA-256 | Embedded RGBA SHA-256 |
| --- | --- | --- |
| Blue Marble | `5b62bc51015e2eaee1978518b2c7e60ec67271608c6d6a8ce2a7bdc396af476f` | `638101ffd167908ec336675736c827128ab050c8b0abf5ebb3f1875a4a0ceb8e` |
| Video poster | `81e1b614a7f35cdd411ff2b0db5a3abf9221ee0a0c86340831ce744f7b3eed24` | `818a6b366100dd85a1ad0c752035457215cced8885d5bbd99c89aaf732711768` |

The Gallery generates the separate color test image in Rust. Its outside bands, four middle colors, and one translucent patch make crop, orientation, corners, and alpha visible in screenshots.
