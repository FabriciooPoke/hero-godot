# AU (AUv2) do BC Drums: o clap-wrapper embrulha o "BC Drums.clap" gerado pelo
# nih-plug num "BC Drums.component" para o Logic e outros hosts AU.
cmake_minimum_required(VERSION 3.21)
set(CMAKE_OSX_ARCHITECTURES "arm64;x86_64" CACHE STRING "")
set(CMAKE_OSX_DEPLOYMENT_TARGET "11.0" CACHE STRING "")
project(bcdrums_au VERSION 0.1.0 LANGUAGES C CXX OBJC OBJCXX)
set(CMAKE_CXX_STANDARD 17)

set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES ON CACHE BOOL "" FORCE)
set(CLAP_WRAPPER_BUILD_AUV2 ON CACHE BOOL "" FORCE)
include(FetchContent)
FetchContent_Declare(clapwrapper
  GIT_REPOSITORY https://github.com/free-audio/clap-wrapper.git
  GIT_TAG main
  GIT_SHALLOW TRUE)
FetchContent_MakeAvailable(clapwrapper)

if(NOT DEFINED BCD_CLAP)
  message(FATAL_ERROR "informe -DBCD_CLAP=<caminho do BC Drums.clap>")
endif()

add_library(bcdrums_auv2 MODULE)
target_add_auv2_wrapper(
  TARGET bcdrums_auv2
  OUTPUT_NAME "BC Drums"
  BUNDLE_IDENTIFIER "br.com.bcstudio.bcdrums.auv2"
  BUNDLE_VERSION "0.1.0"
  MANUFACTURER_NAME "BC Studio"
  MANUFACTURER_CODE "BCst"
  SUBTYPE_CODE "BcDr"
  INSTRUMENT_TYPE "aufx"
  MACOS_EMBEDDED_CLAP_LOCATION "${BCD_CLAP}")

# leva o .clap para dentro do .component (Contents/PlugIns)
add_custom_command(TARGET bcdrums_auv2 POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E make_directory "$<TARGET_BUNDLE_DIR:bcdrums_auv2>/Contents/PlugIns"
  COMMAND cp -R "${BCD_CLAP}" "$<TARGET_BUNDLE_DIR:bcdrums_auv2>/Contents/PlugIns/")
