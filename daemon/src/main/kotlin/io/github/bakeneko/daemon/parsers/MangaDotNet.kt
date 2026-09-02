package io.github.bakeneko.daemon.parsers

import io.github.bakeneko.daemon.ChapterDto
import io.github.bakeneko.daemon.MangaDto
import io.github.bakeneko.daemon.PageDto
import io.github.bakeneko.daemon.SourceDto
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.Headers
import okhttp3.OkHttpClient
import okhttp3.Request
import java.net.URLEncoder

class MangaDotNet(private val httpClient: OkHttpClient) {

    companion object {
        const val SOURCE_ID = "MANGADOT_NET"
        const val SOURCE_NAME = "MangaDot"
        const val BASE_URL = "https://mangadot.net"
        const val USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"

        val SOURCE_DTO = SourceDto(
            id = SOURCE_ID,
            name = SOURCE_NAME,
            language = null,
        )
    }

    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    fun getRequestHeaders(): Headers = Headers.Builder()
        .set("User-Agent", USER_AGENT)
        .set("Referer", "$BASE_URL/")
        .set("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8")
        .build()

    private fun apiHeaders(): Headers = Headers.Builder()
        .set("User-Agent", USER_AGENT)
        .set("Referer", "$BASE_URL/")
        .set("Accept", "application/json, text/plain, */*")
        .build()

    private fun executeGet(url: String): String {
        val request = Request.Builder()
            .url(url)
            .headers(apiHeaders())
            .get()
            .build()
        httpClient.newCall(request).execute().use { response ->
            if (!response.isSuccessful) {
                throw RuntimeException("HTTP ${response.code} fetching $url")
            }
            return response.body?.string() ?: ""
        }
    }

    suspend fun catalog(offset: Int, query: String?, categories: List<String>): List<MangaDto> {
        val hasQuery = !query.isNullOrBlank()
        val hasCategories = categories.isNotEmpty()

        val url = if (hasQuery || hasCategories) {
            val page = (offset / 20) + 1
            val params = mutableListOf<String>()
            if (hasQuery) {
                params.add("search=${URLEncoder.encode(query!!.trim(), "UTF-8")}")
            }
            for (cat in categories) {
                if (cat.isNotBlank()) {
                    params.add("genres=${URLEncoder.encode(cat.trim(), "UTF-8")}")
                }
            }
            params.add("page=$page")
            "$BASE_URL/api/search?${params.joinToString("&")}"
        } else {
            val page = (offset / 15) + 1
            "$BASE_URL/api/manga?page=$page"
        }

        val body = executeGet(url)
        if (body.isBlank()) return emptyList()

        val root = json.parseToJsonElement(body).jsonObject
        val listArray = root["manga_list"]?.jsonArray ?: return emptyList()

        val results = mutableListOf<MangaDto>()
        for (item in listArray) {
            val obj = item.jsonObject
            val id = obj["id"]?.jsonPrimitive?.content ?: continue
            val title = obj["title"]?.jsonPrimitive?.content ?: continue
            val photo = obj["photo"]?.jsonPrimitive?.content
            val coverUrl = if (photo?.startsWith("/") == true) "$BASE_URL$photo" else photo

            val isAdult = obj["is_adult"]?.jsonPrimitive?.let { it.content == "1" || it.content.equals("true", ignoreCase = true) } ?: false
                || obj["content_rating"]?.jsonPrimitive?.content?.lowercase() in listOf("pornographic", "erotica", "adult")

            results.add(
                MangaDto(
                    source = SOURCE_ID,
                    url = "/manga/$id",
                    title = title,
                    publicUrl = "$BASE_URL/manga/$id",
                    coverUrl = coverUrl,
                    largeCoverUrl = coverUrl,
                    isNsfw = isAdult,
                )
            )
        }
        return results
    }

    suspend fun details(mangaDto: MangaDto): MangaDto {
        val id = mangaDto.url.removePrefix("/manga/").trim().takeWhile { it.isDigit() }
        if (id.isEmpty()) {
            return mangaDto
        }

        val mangaBody = executeGet("$BASE_URL/api/manga/$id")
        val mangaRoot = json.parseToJsonElement(mangaBody).jsonObject
        val mangaObj = mangaRoot["manga"]?.jsonObject ?: mangaRoot

        val title = mangaObj["title"]?.jsonPrimitive?.content ?: mangaDto.title
        val photo = mangaObj["photo"]?.jsonPrimitive?.content
        val coverUrl = if (photo?.startsWith("/") == true) "$BASE_URL$photo" else (photo ?: mangaDto.coverUrl)
        val description = mangaObj["description"]?.jsonPrimitive?.content ?: mangaDto.description
        val status = mangaObj["status"]?.jsonPrimitive?.content?.uppercase()

        val isAdult = mangaObj["is_adult"]?.jsonPrimitive?.let { it.content == "1" || it.content.equals("true", ignoreCase = true) } ?: false
            || mangaObj["content_rating"]?.jsonPrimitive?.content?.lowercase() in listOf("pornographic", "erotica", "adult")

        val authors = mutableListOf<String>()
        mangaObj["authors"]?.let { el ->
            val raw = el.jsonPrimitive.content
            if (raw.startsWith("[")) {
                try {
                    val arr = json.parseToJsonElement(raw).jsonArray
                    for (a in arr) {
                        authors.add(a.jsonPrimitive.content)
                    }
                } catch (_: Exception) {
                    authors.add(raw)
                }
            } else if (raw.isNotBlank()) {
                authors.add(raw)
            }
        }

        // Obtener capítulos
        val chaptersList = mutableListOf<ChapterDto>()
        try {
            val chaptersBody = executeGet("$BASE_URL/api/manga/$id/chapters/list")
            val chaptersArray = json.parseToJsonElement(chaptersBody).jsonArray
            for (chItem in chaptersArray) {
                val ch = chItem.jsonObject
                val chId = ch["id"]?.jsonPrimitive?.content ?: continue
                val chNum = ch["chapter_number"]?.jsonPrimitive?.content?.toFloatOrNull() ?: 0f
                val chVol = ch["volume_number"]?.jsonPrimitive?.content?.toIntOrNull() ?: 0
                val rawTitle = ch["chapter_title"]?.jsonPrimitive?.content?.trim()
                val displayTitle = if (!rawTitle.isNullOrBlank()) {
                    rawTitle
                } else if (chNum % 1f == 0f) {
                    "Capítulo ${chNum.toInt()}"
                } else {
                    "Capítulo $chNum"
                }

                val language = ch["language"]?.jsonPrimitive?.content
                val scanlator = ch["scanlator_name"]?.jsonPrimitive?.content

                chaptersList.add(
                    ChapterDto(
                        source = SOURCE_ID,
                        url = "/chapter/$chId",
                        title = displayTitle,
                        number = chNum,
                        volume = chVol,
                        language = language,
                        scanlator = scanlator,
                        uploadDate = 0L,
                        branch = language,
                    )
                )
            }
        } catch (e: Exception) {
            System.err.println("Error fetching MangaDot chapters for $id: ${e.message}")
        }

        return MangaDto(
            source = SOURCE_ID,
            url = mangaDto.url,
            title = title,
            publicUrl = "$BASE_URL/manga/$id",
            coverUrl = coverUrl,
            largeCoverUrl = coverUrl,
            description = description,
            authors = authors,
            state = status,
            isNsfw = isAdult,
            chapters = chaptersList,
        )
    }

    suspend fun pages(chapterDto: ChapterDto): List<PageDto> {
        val id = chapterDto.url.removePrefix("/chapter/").trim().takeWhile { it.isDigit() }
        if (id.isEmpty()) {
            return emptyList()
        }

        val urlsToTry = listOf(
            "$BASE_URL/api/uploads/$id/images",
            "$BASE_URL/api/chapters/$id/images",
        )

        for (apiUrl in urlsToTry) {
            try {
                val body = executeGet(apiUrl)
                if (body.isNotBlank()) {
                    val root = json.parseToJsonElement(body).jsonObject
                    val imagesArray = root["images"]?.jsonArray
                    if (imagesArray != null && imagesArray.isNotEmpty()) {
                        val pages = mutableListOf<PageDto>()
                        for (imgItem in imagesArray) {
                            val imgObj = imgItem.jsonObject
                            val rawUrl = imgObj["url"]?.jsonPrimitive?.content ?: continue
                            val fullUrl = if (rawUrl.startsWith("/")) "$BASE_URL$rawUrl" else rawUrl
                            pages.add(
                                PageDto(
                                    source = SOURCE_ID,
                                    url = fullUrl,
                                )
                            )
                        }
                        if (pages.isNotEmpty()) {
                            return pages
                        }
                    }
                }
            } catch (_: Exception) {
                // Siguiente URL de fallback
            }
        }

        return emptyList()
    }

    suspend fun pageUrl(pageDto: PageDto): String = pageDto.url
}
