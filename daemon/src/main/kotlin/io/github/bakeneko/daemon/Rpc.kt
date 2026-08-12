package io.github.bakeneko.daemon

import kotlinx.serialization.Serializable

/** Error estándar de JSON-RPC que aborta una request individual. */
class RpcError(val code: Int, message: String, cause: Throwable? = null) :
    RuntimeException(message, cause)

@Serializable
data class RpcResponseError(val code: Int, val message: String)

object RpcCodes {
    const val PARSE_ERROR = -32700
    const val INVALID_REQUEST = -32600
    const val METHOD_NOT_FOUND = -32601
    const val INVALID_PARAMS = -32602
    const val INTERNAL_ERROR = -32603
}