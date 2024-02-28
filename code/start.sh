#!/bin/bash
SHELL_FOLDER=$(cd "$(dirname "$0")";pwd)

CUDA_LIBPATH=`python3 -c 'import os; import nvidia.cublas.lib; import nvidia.cudnn.lib; print(os.path.dirname(nvidia.cublas.lib.__file__) + ":" + os.path.dirname(nvidia.cudnn.lib.__file__))'` && export LD_LIBRARY_PATH=$CUDA_LIBPATH:$LD_LIBRARY_PATH
python3 code/app.py

#while true; do sleep 10; done