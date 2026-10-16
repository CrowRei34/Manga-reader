# CPack invokes this script after installing the package tree. Normalizing
# mtimes makes the archive independent from the time of the build while still
# allowing SOURCE_DATE_EPOCH to be omitted for ad-hoc local builds.
if(DEFINED ENV{SOURCE_DATE_EPOCH} AND NOT "$ENV{SOURCE_DATE_EPOCH}" STREQUAL "")
  execute_process(
    COMMAND find "${CMAKE_INSTALL_PREFIX}" -exec touch -h -d "@$ENV{SOURCE_DATE_EPOCH}" "{}" "+"
    RESULT_VARIABLE result
    OUTPUT_QUIET
    ERROR_VARIABLE error_output)
  if(NOT result EQUAL 0)
    message(FATAL_ERROR "No se pudieron normalizar las fechas del paquete: ${error_output}")
  endif()
endif()
