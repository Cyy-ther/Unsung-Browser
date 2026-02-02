cmake_minimum_required(VERSION 3.10)
project(FetchServer)

set(CMAKE_CXX_STANDARD 17)

find_package(CURL REQUIRED)

include_directories(${CMAKE_SOURCE_DIR})

add_executable(FetchServer main.cpp)

target_link_libraries(FetchServer PRIVATE CURL::libcurl)
