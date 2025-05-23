[![crates.io](https://img.shields.io/crates/v/bevy_gstreamer)](https://crates.io/crates/bevy_gstreamer)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/Seldom-SE/seldom_pixel#license)
[![crates.io](https://img.shields.io/crates/d/bevy_gstreamer)](https://crates.io/crates/bevy_gstreamer)
[![CI](https://github.com/foxzool/bevy_gstreamer/workflows/CI/badge.svg)](https://github.com/foxzool/bevy_gstreamer/actions)
[![Documentation](https://docs.rs/bevy_gstreamer/badge.svg)](https://docs.rs/bevy_gstreamer)

# `bevy_gstreamer`

![2023-04-27 180916](https://user-images.githubusercontent.com/217027/234832021-cfd3cf1d-9c26-4e63-b7c1-6120949bd431.png)

This crate provide a gstreamer pipeline to render webcamera to bevy render background.

--------

# Install dependency

## Linux/BSDs

```bash
$ apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
      gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev
```

## Homebrew

```bash
$ brew install gstreamer gst-plugins-base gst-plugins-good \
      gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server \
      gst-editing-services --with-orc --with-libogg --with-opus \
      --with-pango --with-theora --with-libvorbis --with-libvpx \
      --enable-gtk3


```

# Support

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

| bevy | bevy_gstreamer |
|------|----------------|
| 0.16 | 0.7            |
| 0.15 | 0.6            |
| 0.14 | 0.5            |
| 0.13 | 0.4            |
| 0.12 | 0.3            |
| 0.11 | 0.2            |
| 0.10 | 0.1            |

## License

Dual-licensed under either:

- [`MIT`](LICENSE-MIT): [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT)
- [`Apache 2.0`](LICENSE-APACHE): [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)

At your option. This means that when using this crate in your game, you may choose which license to use.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.
