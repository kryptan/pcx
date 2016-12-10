Using bash:

  1. Download pcx samples:
   
    rsync -aL "rsync://samples.libav.org/samples/image-samples/pcx" .
    
  2. Check download:
  
    cd pcx
    md5sum -c md5sum
    cd cga
    md5sum -c md5sum
    cd ../..
 
  2. Convert them to BMPs:

    for i in pcx/*.pcx; do ffmpeg -pix_fmt rgb24 -i "$i" "${i%.pcx}.bmp"; done
    for i in pcx/*.PCX; do ffmpeg -pix_fmt rgb24 -i "$i" "${i%.PCX}.bmp"; done
    for i in pcx/cga/*.PCX; do ffmpeg -pix_fmt rgb24 -i "$i" "${i%.PCX}.bmp"; done

Set environment variable 