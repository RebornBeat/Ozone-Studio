# Install script for directory: /home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "RelWithDebInfo")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Install shared libraries without execute permission?
if(NOT DEFINED CMAKE_INSTALL_SO_NO_EXE)
  set(CMAKE_INSTALL_SO_NO_EXE "1")
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

# Set default install directory permissions.
if(NOT DEFINED CMAKE_OBJDUMP)
  set(CMAKE_OBJDUMP "/usr/bin/objdump")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/build/ggml/src/cmake_install.cmake")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/build/ggml/src/libggml.a")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-alloc.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-backend.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-blas.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-cann.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-cuda.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-kompute.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-metal.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-rpc.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-sycl.h"
    "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/voice/target/debug/build/whisper-rs-sys-ea6ce2d4ee4505a8/out/whisper.cpp/ggml/include/ggml-vulkan.h"
    )
endif()

