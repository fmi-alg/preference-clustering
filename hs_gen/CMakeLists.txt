# This is the CMake script for compiling a CGAL application.

cmake_minimum_required(VERSION 3.1...3.15)
project (hs_gen)

if(NOT POLICY CMP0070 AND POLICY CMP0053)
  # Only set CMP0053 to OLD with CMake<3.10, otherwise there is a warning.
  cmake_policy(SET CMP0053 OLD)
endif()

if(POLICY CMP0071)
  cmake_policy(SET CMP0071 NEW)
endif()

find_package(OpenMP)


find_package(CGAL COMPONENTS Qt5)

find_package(Qt5 QUIET COMPONENTS Xml Script OpenGL Svg)



if ( CGAL_FOUND AND CGAL_Qt5_FOUND AND Qt5_FOUND )

  add_definitions(-DQT_NO_KEYWORDS)
  set(CMAKE_INCLUDE_CURRENT_DIR ON)

  add_executable  ( arrang arrang.cpp  ${CGAL_Qt5_RESOURCE_FILES} ${CGAL_Qt5_MOC_FILES})
  add_executable  ( SetMinimizer SetMinimizer.cpp )
  add_executable  ( contained contained.cpp )

  add_to_cached_list( CGAL_EXECUTABLE_TARGETS generator cli_demo arrang_demo_ui)


  target_link_libraries( arrang PRIVATE
    CGAL::CGAL CGAL::CGAL_Qt5 Qt5::Gui OpenMP::OpenMP_CXX)

  target_link_libraries( SetMinimizer PRIVATE OpenMP::OpenMP_CXX )

  target_link_libraries( contained PRIVATE CGAL::CGAL )

  include(${CGAL_MODULES_DIR}/CGAL_add_test.cmake)
  cgal_add_compilation_test(arrang)
  cgal_add_compilation_test(SetMinimizer)
else()

  message(STATUS "NOTICE: This application requires CGAL and Qt5, and can not be compiled without them.")
  message(STATUS "Status of Dependencies:")
  message(STATUS "CGAL_FOUND: " ${CGAL_FOUND})
  message(STATUS "CGAL_Qt5_FOUND: " ${CGAL_Qt5_FOUND})
  message(STATUS "Qt5_FOUND: " ${Qt5_FOUND})

endif()
