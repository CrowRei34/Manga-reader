cmake_minimum_required(VERSION 3.24)
project(BakenekoPackage LANGUAGES NONE)

set(BAKENEKO_VERSION "0.1.0" CACHE STRING "Versión de Bakeneko")
set(BAKENEKO_JRE_HOME "" CACHE PATH "Runtime Java reducido que se incluirá en el paquete")

if(BAKENEKO_JRE_HOME STREQUAL "")
  message(FATAL_ERROR "BAKENEKO_JRE_HOME está vacío; no se puede crear un paquete sin Java incluido")
endif()

foreach(required_file
    "${CMAKE_SOURCE_DIR}/target/release/bakeneko"
    "${CMAKE_SOURCE_DIR}/daemon/build/libs/bakeneko-daemon.jar"
    "${BAKENEKO_JRE_HOME}/bin/java"
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
# Do not inherit permissions from the JDK installation. Some CI caches expose
# the JDK as 0777, which would otherwise leak world-writable files into the
# release archive. Regular runtime files are 0644 and directories are 0755.
install(DIRECTORY "${BAKENEKO_JRE_HOME}/" DESTINATION app/jre
    FILE_PERMISSIONS
      OWNER_READ OWNER_WRITE
      GROUP_READ
      WORLD_READ
    DIRECTORY_PERMISSIONS
      OWNER_READ OWNER_WRITE OWNER_EXECUTE
      GROUP_READ GROUP_EXECUTE
      WORLD_READ WORLD_EXECUTE)
# java and the other launchers must remain executable after the permission
# normalization above.
install(DIRECTORY "${BAKENEKO_JRE_HOME}/bin/" DESTINATION app/jre/bin
    FILE_PERMISSIONS
      OWNER_READ OWNER_WRITE OWNER_EXECUTE
      GROUP_READ GROUP_EXECUTE
      WORLD_READ WORLD_EXECUTE
    DIRECTORY_PERMISSIONS
      OWNER_READ OWNER_WRITE OWNER_EXECUTE
      GROUP_READ GROUP_EXECUTE
      WORLD_READ WORLD_EXECUTE)
foreach(runtime_helper jexec jspawnhelper)
  if(EXISTS "${BAKENEKO_JRE_HOME}/lib/${runtime_helper}")
    install(PROGRAMS "${BAKENEKO_JRE_HOME}/lib/${runtime_helper}" DESTINATION app/jre/lib)
  endif()
endforeach()
install(CODE "file(REMOVE \"\$ENV{DESTDIR}\${CMAKE_INSTALL_PREFIX}/app/jre/lib/security/cacerts\")")
install(FILES "${BAKENEKO_CACERTS}" DESTINATION app/jre/lib/security RENAME cacerts)
install(SCRIPT "${CMAKE_SOURCE_DIR}/packaging/normalize-install.cmake")

set(CPACK_GENERATOR "TGZ")
set(CPACK_PACKAGE_NAME "Bakeneko-Portable")
set(CPACK_PACKAGE_VENDOR "Bakeneko")
set(CPACK_PACKAGE_VERSION "${BAKENEKO_VERSION}")
set(CPACK_PACKAGE_FILE_NAME "Bakeneko-Portable-v${BAKENEKO_VERSION}-Linux-x86_64")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Lector de manga portable para Linux x86_64")
set(CPACK_VERBATIM_VARIABLES ON)
set(CPACK_ARCHIVE_COMPONENT_INSTALL OFF)
set(CPACK_INCLUDE_TOPLEVEL_DIRECTORY ON)
include(CPack)
