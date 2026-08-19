PHP_ARG_ENABLE([raddy],
  [whether to enable the raddy extension],
  [AS_HELP_STRING([--enable-raddy], [Enable raddy])],
  [yes])

if test "$PHP_RADDY" != "no"; then
  PHP_NEW_EXTENSION(raddy, raddy.c, $ext_shared,, -DZEND_ENABLE_STATIC_TSRMLS_CACHE=1)
fi
