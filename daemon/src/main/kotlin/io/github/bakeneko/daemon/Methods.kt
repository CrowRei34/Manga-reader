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
import io.github.bakeneko.daemon.parsers.MangaDotNet
import io.github.landwarderer.futon.parsers.model.MangaListFilter
import io.github.landwarderer.futon.parsers.model.MangaParserSource
import io.github.landwarderer.futon.parsers.model.SortOrder

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
    private val mangaDotNet = MangaDotNet(ctx.httpClient)

    /** Devuelve el JsonElement del `result`. Lanza [RpcError] en fallo. */
    suspend fun invoke(method: String, params: JsonObject?): JsonElement = when (method) {
        "ping" -> buildJsonObject {
            put("version", DaemonVersion)
            put("java", System.getProperty("java.version"))
        }

        "sources.list" -> {
            val official = MangaParserSource.entries.map { it.toDto() }
            val allSources = listOf(MangaDotNet.SOURCE_DTO) + official
            json.encodeToJsonElement(allSources)
        }

        "catalog.list" -> {
            val sourceId = params.sourceId()
            val offset = params.intOrDefault("offset", 0)
            val query = params?.optionalString("query")
            val categories = params?.get("categories")?.jsonArray
                ?.mapNotNull { (it as? JsonPrimitive)?.content }
                ?: emptyList()
            if (sourceId == MangaDotNet.SOURCE_ID) {
                json.encodeToJsonElement(mangaDotNet.catalog(offset, query, categories))
            } else {
                val source = params.source()
                catalog(source, offset, query, categories)
            }
        }

        "manga.details" -> {
            val sourceId = params.sourceId()
            val manga = params.reqObj("manga").decode<MangaDto>()
            if (sourceId == MangaDotNet.SOURCE_ID) {
                json.encodeToJsonElement(mangaDotNet.details(manga))
            } else {
                val source = params.source()
                json.encodeToJsonElement(details(source, manga))
            }
        }

        "chapter.pages" -> {
            val sourceId = params.sourceId()
            val chapter = params.reqObj("chapter").decode<ChapterDto>()
            if (sourceId == MangaDotNet.SOURCE_ID) {
                json.encodeToJsonElement(mangaDotNet.pages(chapter))
            } else {
                val source = params.source()
                json.encodeToJsonElement(pages(source, chapter))
            }
        }

        "page.url" -> {
            val sourceId = params.sourceId()
            val page = params.reqObj("page").decode<PageDto>()
            if (sourceId == MangaDotNet.SOURCE_ID) {
                JsonPrimitive(mangaDotNet.pageUrl(page))
            } else {
                val source = params.source()
                JsonPrimitive(pages_url(source, page))
            }
        }

        "source.headers" -> {
            val sourceId = params.sourceId()
            val headers = if (sourceId == MangaDotNet.SOURCE_ID) {
                mangaDotNet.getRequestHeaders()
            } else {
                val source = params.source()
                val parser = getParser(source)
                parser.getRequestHeaders()
            }
            buildJsonObject {
                for (i in 0 until headers.size) {
                    put(headers.name(i), headers.value(i))
                }
            }
        }

        else -> throw RpcError(-32601, "método desconocido: $method")
    }

    private fun JsonObject?.sourceId(): String {
        return this?.optionalString("source") ?: throw RpcError(-32602, "falta source")
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

    private val parserCache = java.util.concurrent.ConcurrentHashMap<MangaParserSource, io.github.landwarderer.futon.parsers.MangaParser>()

    private fun getParser(source: MangaParserSource, reset: Boolean = false): io.github.landwarderer.futon.parsers.MangaParser {
        if (reset) {
            val p = ctx.newParserInstance(source)
            parserCache[source] = p
            return p
        }
        return parserCache.computeIfAbsent(source) { ctx.newParserInstance(it) }
    }

    private suspend fun catalog(
        source: MangaParserSource, offset: Int, query: String?, categories: List<String>,
    ): JsonElement {
        val parser = getParser(source, reset = (offset == 0))
        val order = if (parser.availableSortOrders.contains(SortOrder.POPULARITY)) {
            SortOrder.POPULARITY
        } else {
            parser.availableSortOrders.firstOrNull() ?: SortOrder.UPDATED
        }
        val requested = categories.flatMap(::categoryAliases).map { it.lowercase() }.toSet()
        val tags = if (requested.isEmpty()) {
            emptySet()
        } else {
            parser.getFilterOptions().availableTags.filterTo(linkedSetOf()) { tag ->
                val title = tag.title.lowercase()
                val key = tag.key.lowercase()
                requested.any { alias -> title == alias || key == alias || title.contains(alias) }
            }
        }
        val filter = if (query.isNullOrBlank() && tags.isEmpty()) {
            MangaListFilter.EMPTY
        } else {
            MangaListFilter(query = query, tags = tags)
        }
        val list = parser.getList(offset, order, filter)
        return json.encodeToJsonElement(list.map { it.toDto() })
    }

    private fun categoryAliases(category: String): List<String> = when (category) {
        "action" -> listOf("action", "acción", "acao", "ação")
        "adventure" -> listOf("adventure", "aventura")
        "comedy" -> listOf("comedy", "comedia", "comédie")
        "drama" -> listOf("drama", "drame")
        "fantasy" -> listOf("fantasy", "fantasía", "fantasia", "fantastique")
        "romance" -> listOf("romance", "romántico", "romantico")
        "school" -> listOf("school", "school life", "escolar", "vida escolar")
        "mystery" -> listOf("mystery", "misterio", "mystère")
        "horror" -> listOf("horror", "terror", "horreur")
        "sci-fi" -> listOf("sci-fi", "science fiction", "ciencia ficción", "sci fi")
        "sports" -> listOf("sports", "deportes", "sport")
        "isekai" -> listOf("isekai")
        "slice-of-life" -> listOf("slice of life", "recuentos de la vida", "tranche de vie")
        "yaoi" -> listOf("yaoi", "boys love", "boy's love", "bl")
        "yuri" -> listOf("yuri", "girls love", "girl's love", "gl")
        "ecchi" -> listOf("ecchi")
        "hentai" -> listOf("hentai", "adult", "18+")
        else -> listOf(category)
    }

    private suspend fun details(source: MangaParserSource, mangaDto: MangaDto): MangaDto {
        val parser = getParser(source)
        val detailed = parser.getDetails(mangaDto.toModel())
        return detailed.toDto()
    }

    private suspend fun pages(source: MangaParserSource, chapterDto: ChapterDto): List<PageDto> {
        val parser = getParser(source)
        val list = parser.getPages(chapterDto.toModel())
        return list.map { it.toDto() }
    }

    private suspend fun pages_url(source: MangaParserSource, pageDto: PageDto): String {
        val parser = getParser(source)
        return parser.getPageUrl(pageDto.toModel())
    }
}

const val DaemonVersion = "1.0.0"
