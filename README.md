# asr_server in Rust.

~~~
cd asr_server/whisper-rs/sys/whisper.cpp
bash ./models/download-ggml-model.sh large-v3

make quantize
./quantize models/ggml-large-v3.bin models/ggml-large-v3-q5_0.bin q5_0
~~~
