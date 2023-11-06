# `bevy_gstreamer`
![屏幕截图 2023-04-27 180916](https://user-images.githubusercontent.com/217027/234832021-cfd3cf1d-9c26-4e63-b7c1-6120949bd431.png)



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
##  Homebrew
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
| 0.12 | 0.3.X          |
| 0.11 | 0.2.X          |
| 0.10 | 0.1.X          |

# Licensing
The project is under dual license MIT and Apache 2.0, so join to your hearts content, just remember the license agreements.

# Contributing
Yes this project is still very much WIP, so PRs are very welcome
