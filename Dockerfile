#FROM ubuntu:22.04
FROM nvidia/cuda:11.8.0-cudnn8-runtime-ubuntu22.04
WORKDIR /work/

ENV TZ=Asia/Shanghai
ENV LANG=C.UTF-8 LC_ALL=C.UTF-8
ARG USER=tts
ARG USERPWD=tts

RUN sed -i s@archive.ubuntu.com@mirrors.aliyun.com@g /etc/apt/sources.list
RUN sed -i s@security.ubuntu.com@mirrors.aliyun.com@g /etc/apt/sources.list

RUN apt-get clean
RUN apt-get update --fix-missing
RUN apt-get install -y tzdata \
    && ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
    && dpkg-reconfigure -f noninteractive tzdata
RUN apt-get install -y fonts-wqy-zenhei apt-utils pkg-config openssl libssl-dev curl ffmpeg
RUN apt-get install -y python3 python3-pip
RUN apt-get clean autoclean
RUN apt-get autoremove --yes
RUN rm -rf /var/lib/{apt,dpkg,cache,log}/

RUN apt-get install -y sudo
RUN adduser --ingroup sudo --disabled-password --gecos "" --shell /bin/bash --home /home/${USER} ${USER} && \
    usermod -aG sudo ${USER} && \
    echo "%sudo  ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/nopasswd
RUN echo "$USER:$USERPWD" | chpasswd

COPY ./code /work/code
COPY ./faster-whisper-large-v3 /work/faster-whisper-large-v3
COPY ./templates /work/templates

RUN pip install --no-cache-dir -r /work/code/requirements.txt  -i https://mirrors.aliyun.com/pypi/simple/

CMD ["bash", "/work/code/start.sh"]