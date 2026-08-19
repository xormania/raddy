/*
 * Userland surface for the C3 hostcalls. Built only with the custom SAPI
 * (ADR 0008). Function names match proj/plan.md §C3.
 */
#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

#include "php.h"
#include "php_raddy.h"

static PHP_FUNCTION(head);
static PHP_FUNCTION(body_read);
static PHP_FUNCTION(resp_head);
static PHP_FUNCTION(write);
static PHP_FUNCTION(end);
static PHP_FUNCTION(log_msg);
static PHP_FUNCTION(cap);

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_head, 0, 0, IS_STRING, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_body_read, 0, 1, IS_STRING, 1)
    ZEND_ARG_TYPE_INFO(0, max, IS_LONG, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_resp_head, 0, 1, IS_VOID, 0)
    ZEND_ARG_TYPE_INFO(0, json, IS_STRING, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_write, 0, 1, IS_VOID, 0)
    ZEND_ARG_TYPE_INFO(0, chunk, IS_STRING, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_end, 0, 0, IS_VOID, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_log_msg, 0, 2, IS_VOID, 0)
    ZEND_ARG_TYPE_INFO(0, level, IS_LONG, 0)
    ZEND_ARG_TYPE_INFO(0, msg, IS_STRING, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_cap, 0, 2, IS_STRING, 0)
    ZEND_ARG_TYPE_INFO(0, ns, IS_STRING, 0)
    ZEND_ARG_TYPE_INFO(0, reqJson, IS_STRING, 0)
ZEND_END_ARG_INFO()

static const zend_function_entry raddy_functions[] = {
    ZEND_NS_NAMED_FE("Raddy", head, ZEND_FN(head), arginfo_head)
    ZEND_NS_NAMED_FE("Raddy", body_read, ZEND_FN(body_read), arginfo_body_read)
    ZEND_NS_NAMED_FE("Raddy", resp_head, ZEND_FN(resp_head), arginfo_resp_head)
    ZEND_NS_NAMED_FE("Raddy", write, ZEND_FN(write), arginfo_write)
    ZEND_NS_NAMED_FE("Raddy", end, ZEND_FN(end), arginfo_end)
    ZEND_NS_NAMED_FE("Raddy", log, ZEND_FN(log_msg), arginfo_log_msg)
    ZEND_NS_NAMED_FE("Raddy", cap, ZEND_FN(cap), arginfo_cap)
    PHP_FE_END
};

static PHP_FUNCTION(head)
{
    ZEND_PARSE_PARAMETERS_NONE();
    zend_throw_error(NULL, "Raddy\\head requires the raddy SAPI");
}

static PHP_FUNCTION(body_read)
{
    zend_long max;
    ZEND_PARSE_PARAMETERS_START(1, 1)
        Z_PARAM_LONG(max)
    ZEND_PARSE_PARAMETERS_END();
    zend_throw_error(NULL, "Raddy\\body_read requires the raddy SAPI");
}

static PHP_FUNCTION(resp_head)
{
    zend_string *json;
    ZEND_PARSE_PARAMETERS_START(1, 1)
        Z_PARAM_STR(json)
    ZEND_PARSE_PARAMETERS_END();
    zend_throw_error(NULL, "Raddy\\resp_head requires the raddy SAPI");
}

static PHP_FUNCTION(write)
{
    zend_string *chunk;
    ZEND_PARSE_PARAMETERS_START(1, 1)
        Z_PARAM_STR(chunk)
    ZEND_PARSE_PARAMETERS_END();
    zend_throw_error(NULL, "Raddy\\write requires the raddy SAPI");
}

static PHP_FUNCTION(end)
{
    ZEND_PARSE_PARAMETERS_NONE();
    zend_throw_error(NULL, "Raddy\\end requires the raddy SAPI");
}

static PHP_FUNCTION(log_msg)
{
    zend_long level;
    zend_string *msg;
    ZEND_PARSE_PARAMETERS_START(2, 2)
        Z_PARAM_LONG(level)
        Z_PARAM_STR(msg)
    ZEND_PARSE_PARAMETERS_END();
    zend_throw_error(NULL, "Raddy\\log requires the raddy SAPI");
}

static PHP_FUNCTION(cap)
{
    zend_string *ns;
    zend_string *req;
    ZEND_PARSE_PARAMETERS_START(2, 2)
        Z_PARAM_STR(ns)
        Z_PARAM_STR(req)
    ZEND_PARSE_PARAMETERS_END();
    zend_throw_error(NULL, "Raddy\\cap requires the raddy SAPI");
}

zend_module_entry raddy_module_entry = {
    STANDARD_MODULE_HEADER,
    "raddy",
    raddy_functions,
    NULL,
    NULL,
    NULL,
    NULL,
    NULL,
    PHP_RADDY_VERSION,
    STANDARD_MODULE_PROPERTIES
};
