cmake_minimum_required(VERSION 3.24)
project(BakenekoPackage LANGUAGES NONE)

set(BAKENEKO_VERSION "0.1.0" CACHE STRING "Versión de Bakeneko")
set(BAKENEKO_JRE_HOME "" CACHE PATH "JRE 21 que se incluirá en el paquete")

foreach(required_file
    "${CMAKE_SOURCE_DIR}/target/release/bakeneko"
    "${CMAKE_SOURCE_DIR}/daemon/build/libs/bakeneko-daemon.jar"
    "${BAKENEKO_JRE_HOME}/bin/java"
    "${BAKENEKO_JRE_HOME}/bin/keytool"
    "${BAKENEKO_JRE_HOME}/lib/security/cacerts")
  if(NOT EXISTS "${required_file}")
    message(FATAL_ERROR "Falta un archivo necesario: ${required_file}")
  endif()
endforeach()

# setup-java puede exponer cacerts como symlink absoluto. CPack debe guardar el
# archivo real para que HTTPS funcione después de descomprimir en otra máquina.
file(REAL_PATH "${BAKENEKO_JRE_HOME}/lib/security/cacerts" BAKENEKO_CACERTS)

install(PROGRAMS packaging/bakeneko DESTINATION .)
install(FILES packaging/README-portable.txt DESTINATION . RENAME README.txt)
install(PROGRAMS target/release/bakeneko DESTINATION app)
install(FILES daemon/build/libs/bakeneko-daemon.jar DESTINATION app)
install(DIRECTORY assets DESTINATION app)
install(DIRECTORY "${BAKENEKO_JRE_HOME}/" DESTINATION app/jre USE_SOURCE_PERMISSIONS)
install(CODE "file(REMOVE \"\$ENV{DESTDIR}\${CMAKE_INSTALL_PREFIX}/app/jre/lib/security/cacerts\")")
install(FILES "${BAKENEKO_CACERTS}" DESTINATION app/jre/lib/security RENAME cacerts)

set(CPACK_GENERATOR "TGZ")
set(CPACK_PACKAGE_NAME "Bakeneko-Portable")
set(CPACK_PACKAGE_VENDOR "Bakeneko")
set(CPACK_PACKAGE_VERSION "${BAKENEKO_VERSION}")
set(CPACK_PACKAGE_FILE_NAME "Bakeneko-Portable-v${BAKENEKO_VERSION}-Linux-x86_64")
set(CPACK_ARCHIVE_COMPONENT_INSTALL OFF)
set(CPACK_INCLUDE_TOPLEVEL_DIRECTORY ON)
include(CPack)
