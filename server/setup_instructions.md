# Build Instructions
---
Note: If using Windows, I recommend using Windows Subsystem for Linux (WSL), which is how my environment is set up.

1. (Optional) Create a python virtual environment
2. In llava_video folder, run `pip install -e .[train]`
3. I had to install flash-attn manually, using `pip install https://github.com/Dao-AILab/flash-attention/releases/download/v2.7.2.post1/flash_attn-2.7.2.post1+cu11torch2.1cxx11abiFALSE-cp310-cp310-linux_x86_64.whl` (or the latest release URL for PyTorch 2.1 from https://github.com/Dao-AILab/flash-attention/releases/)
4. If using NVIDIA CUDA, ensure drivers and CUDA Toolkit are installed.  
    - I had to install CUDA 11 for compatability with this project's software versions. 
        - I installed CUDA 11 for WSL here: https://developer.nvidia.com/cuda-11-8-0-download-archive?target_os=Linux&target_arch=x86_64&Distribution=WSL-Ubuntu&target_version=2.0&target_type=deb_local
            - For last step I installed using `sudo apt-get -y install cuda-toolkit-11-8`
    - Make sure when you run `test_cuda.ipynb` that you get an output of your NVIDIA driver information and CUDA version (mine was 11.8).

# Run Instructions
---
1. From the AnyweAR/ directory, run using `python -m server.run_llava_video` 