package io.github.bakeneko.daemon

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import org.koitharu.kotatsu.parsers.model.MangaListFilter
import org.koitharu.kotatsu.parsers.model.MangaParserSource
import org.koitharu.kotatsu.parsers.model.SortOrder

/**
 * Despachador de métodos JSON-RPC. Sin estado: cada call pide un
 * [DaemonLoaderContext] compartido, crea el parser, ejecuta y devuelve
 * un JsonElement resultListo para incrustar en la respuesta.
 *
 * Toda excepción se propaga como [RpcError] para que el bucle del
 * servidor la convierta en una respuesta de error estándar.
 */
class Methods(private val ctx: DaemonLoaderContext) {

    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = false }

    /** Devuelve el JsonElement del `result`. Lanza [RpcError] en fallo. */
    suspend fun invoke(method: String, params: JsonObject?): JsonElement = when (method) {
        "ping" -> buildJsonObject {
            put("version", DaemonVersion)
            put("java", System.getProperty("java.version"))
        }

        "sources.list" ->
            json.encodeToJsonElement(MangaParserSource.entries.map { it.toDto() })

        "catalog.list" -> {
            val source = params.source()
            val offset = params.intOrDefault("offset", 0)
            val query = params?.optionalString("query")
            catalog(source, offset, query)
        }

        "manga.details" -> {
            val source = params.source()
            val manga = params.reqObj("manga").decode<MangaDto>()
            json.encodeToJsonElement(details(source, manga))
        }

        "chapter.pages" -> {
            val source = params.source()
            val chapter = params.reqObj("chapter").decode<ChapterDto>()
            json.encodeToJsonElement(pages(source, chapter))
        }

        "page.url" -> {
            val source = params.source()
            val page = params.reqObj("page").decode<PageDto>()
            JsonPrimitive(pages_url(source, page))
        }

        "source.headers" -> {
            val source = params.source()
            val parser = ctx.newParserInstance(source)
            val headers = parser.getRequestHeaders()
            buildJsonObject {
                for (i in 0 until headers.size) {
                    put(headers.name(i), headers.value(i))
                }
            }
        }

        else -> throw RpcError(-32601, "método desconocido: $method")
    }

    private fun JsonObject?.source(): MangaParserSource {
        val id = this?.optionalString("source") ?: throw RpcError(-32602, "falta source")
        return MangaParserSource.entries.firstOrNull { it.name == id }
            ?: throw RpcError(-32602, "fuente desconocida: $id")
    }

    private fun JsonObject?.reqObj(key: String): JsonObject =
        this?.optionalObj(key) ?: throw RpcError(-32602, "falta el campo '$key'")

    private fun JsonObject?.optionalString(key: String): String? {
        val v = this?.get(key) as? JsonPrimitive ?: return null
        return v.content
    }

    private fun JsonObject?.optionalObj(key: String): JsonObject? = this?.get(key) as? JsonObject

    private fun JsonObject?.intOrDefault(key: String, def: Int): Int =
        this?.get(key)?.jsonPrimitive?.intOrNull ?: def

    private inline fun <reified T> JsonElement.decode(): T = try {
        json.decodeFromJsonElement<T>(this)
    } catch (e: Exception) {
        throw RpcError(-32602, "params inválidos: ${e.message}")
    }

    private suspend fun catalog(
        source: MangaParserSource, offset: Int, query: String?,
    ): JsonElement {
        val parser = ctx.newParserInstance(source)
        val order = parser.availableSortOrders.firstOrNull() ?: SortOrder.UPDATED
        val filter = if (query.isNullOrBlank()) {
            MangaListFilter.EMPTY
        } else {
            MangaListFilter(query = query)
        }
        val list = parser.getList(offset, order, filter)
        return json.encodeToJsonElement(list.map { it.toDto() })
    }

    private suspend fun details(source: MangaParserSource, mangaDto: MangaDto): MangaDto {
        val parser = ctx.newParserInstance(source)
        val detailed = parser.getDetails(mangaDto.toModel())
        return detailed.toDto()
    }

    private suspend fun pages(source: MangaParserSource, chapterDto: ChapterDto): List<PageDto> {
        val parser = ctx.newParserInstance(source)
        val list = parser.getPages(chapterDto.toModel())
        return list.map { it.toDto() }
    }

    private suspend fun pages_url(source: MangaParserSource, pageDto: PageDto): String {
        val parser = ctx.newParserInstance(source)
        return parser.getPageUrl(pageDto.toModel())
    }
}

const val DaemonVersion = "1.0.0"