# Install
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