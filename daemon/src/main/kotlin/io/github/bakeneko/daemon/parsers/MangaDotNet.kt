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

    private fun getSavedCookies(): String? {
        val dataDir = System.getenv("XDG_DATA_HOME") ?: "${System.getenv("HOME")}/.local/share"
        val txtFile = java.io.File("$dataDir/bakeneko/mangadot_cookies.txt")
        if (txtFile.exists()) {
            val content = txtFile.readText().trim()
            if (content.isNotBlank()) return content
        }
        val soupFile = java.io.File("$dataDir/bakeneko/solver_profile/cookies")
        if (soupFile.exists()) {
            val cookies = mutableListOf<String>()
            soupFile.forEachLine { line ->
                if (!line.startsWith("#") || line.startsWith("#HttpOnly_")) {
                    val parts = line.split("\t")
                    if (parts.size >= 7 && parts[0].contains("mangadot.net")) {
                        cookies.add("${parts[5]}=${parts[6]}")
                    }
                }
            }
            if (cookies.isNotEmpty()) {
                return cookies.joinToString("; ")
            }
        }
        return null
    }

    private val client = httpClient.newBuilder()
        .cookieJar(object : okhttp3.CookieJar {
            override fun saveFromResponse(url: okhttp3.HttpUrl, cookies: List<okhttp3.Cookie>) {}
            override fun loadForRequest(url: okhttp3.HttpUrl): List<okhttp3.Cookie> {
                val list = mutableListOf<okhttp3.Cookie>()
                val cookieStr = getSavedCookies() ?: return list
                for (c in cookieStr.split(";")) {
                    val trimmed = c.trim()
                    val eq = trimmed.indexOf('=')
                    if (eq > 0) {
                        val name = trimmed.substring(0, eq)
                        val value = trimmed.substring(eq + 1)
                        try {
                            val cookie = okhttp3.Cookie.Builder()
                                .name(name)
                                .value(value)
                                .domain(url.host)
                                .path("/")
                                .build()
                            list.add(cookie)
                        } catch (_: Exception) {}
                    }
                }
                return list
            }
        })
        .build()

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

    private fun getSolverSocketPath(): String {
        val runtimeDir = System.getenv("XDG_RUNTIME_DIR") ?: run {
            val uid = try {
                java.io.File("/proc/self/status").readLines()
                    .firstOrNull { it.startsWith("Uid:") }
                    ?.split("\\s+".toRegex())?.getOrNull(1) ?: "1000"
            } catch (_: Exception) { "1000" }
            "/tmp/bakeneko-$uid"
        }
        return "$runtimeDir/bakeneko/solver.sock"
    }

    private fun startSolverDaemon() {
        try {
            val solverBin = System.getenv("BAKENEKO_SOLVER_PATH") ?: run {
                val execDir = try {
                    java.io.File(MangaDotNet::class.java.protectionDomain.codeSource.location.toURI()).parentFile
                } catch (_: Exception) { null }
                val paths = mutableListOf(
                    "target/release/bakeneko-solver",
                    "target/debug/bakeneko-solver",
                    "bakeneko-solver",
                    "/usr/local/bin/bakeneko-solver",
                )
                if (execDir != null) {
                    paths.add(java.io.File(execDir, "bakeneko-solver").absolutePath)
                    paths.add(java.io.File(execDir.parentFile, "bakeneko-solver").absolutePath)
                    paths.add(java.io.File(execDir.parentFile?.parentFile, "target/release/bakeneko-solver").absolutePath)
                }
                paths.firstOrNull { java.io.File(it).exists() } ?: "bakeneko-solver"
            }
            val pb = ProcessBuilder(solverBin)
            pb.redirectOutput(ProcessBuilder.Redirect.DISCARD)
            pb.redirectError(ProcessBuilder.Redirect.DISCARD)
            pb.start()

            val sockFile = java.io.File(getSolverSocketPath())
            for (i in 0..50) {
                if (sockFile.exists()) break
                Thread.sleep(100)
            }
        } catch (e: Exception) {
            System.err.println("Error spawning bakeneko-solver daemon: ${e.message}")
        }
    }

    private fun querySolverDaemon(url: String): String {
        val sockPath = getSolverSocketPath()
        val socketFile = java.io.File(sockPath)
        if (!socketFile.exists()) {
            startSolverDaemon()
        }

        try {
            val address = java.net.UnixDomainSocketAddress.of(sockPath)
            java.nio.channels.SocketChannel.open(java.net.StandardProtocolFamily.UNIX).use { channel ->
                channel.connect(address)
                val reader = java.io.BufferedReader(java.nio.channels.Channels.newReader(channel, Charsets.UTF_8))
                val writer = java.io.BufferedWriter(java.nio.channels.Channels.newWriter(channel, Charsets.UTF_8))

                val reqId = java.util.UUID.randomUUID().toString()
                val escapedUrl = url.replace("\\", "\\\\").replace("\"", "\\\"")
                val reqJson = "{\"id\":\"$reqId\",\"url\":\"$escapedUrl\",\"ping\":false}\n"
                writer.write(reqJson)
                writer.flush()

                val responseLine = reader.readLine() ?: return ""
                val root = json.parseToJsonElement(responseLine).jsonObject
                val resultEl = root["result"] ?: return ""
                val resultStr = if (resultEl is kotlinx.serialization.json.JsonPrimitive && resultEl.isString) {
                    resultEl.content
                } else {
                    resultEl.toString()
                }
                if (resultStr.isNotBlank() && (resultStr.startsWith("{") || resultStr.startsWith("["))) {
                    return resultStr
                }
            }
        } catch (e: Exception) {
            System.err.println("querySolverDaemon error: ${e.message}")
        }
        return ""
    }

    private fun executeGet(url: String): String {
        try {
            val request = Request.Builder()
                .url(url)
                .headers(apiHeaders())
                .get()
                .build()
            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    val body = response.body?.string() ?: ""
                    if (body.isNotBlank()) return body
                } else if (response.code == 403 || response.code == 503) {
                    System.err.println("MangaDot HTTP ${response.code} (Cloudflare) on $url -> Delegating to bakeneko-solver daemon...")
                    return querySolverDaemon(url)
                }
            }
        } catch (e: Exception) {
            // Fallback a solver daemon
            return querySolverDaemon(url)
        }
        return querySolverDaemon(url)
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

        return try {
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
                        description = obj["description"]?.jsonPrimitive?.content,
                        authors = emptyList(),
                        state = obj["status"]?.jsonPrimitive?.content?.uppercase(),
                        isNsfw = isAdult,
                        chapters = emptyList(),
                    )
                )
            }
            results
        } catch (e: Exception) {
            System.err.println("Error parsing MangaDot catalog: ${e.message}")
            emptyList()
        }
    }

    suspend fun details(mangaDto: MangaDto): MangaDto {
        val id = mangaDto.url.removePrefix("/manga/").trim().takeWhile { it.isDigit() }
        if (id.isEmpty()) {
            return mangaDto
        }

        var title = mangaDto.title
        var coverUrl = mangaDto.coverUrl
        var description = mangaDto.description
        var status = mangaDto.state
        var isAdult = mangaDto.isNsfw
        val authors = mutableListOf<String>()

        try {
            val mangaBody = executeGet("$BASE_URL/api/manga/$id")
            if (mangaBody.isNotBlank()) {
                val mangaRoot = json.parseToJsonElement(mangaBody).jsonObject
                val mangaObj = mangaRoot["manga"]?.jsonObject ?: mangaRoot

                mangaObj["title"]?.jsonPrimitive?.content?.let { title = it }
                val photo = mangaObj["photo"]?.jsonPrimitive?.content
                if (photo != null) {
                    coverUrl = if (photo.startsWith("/")) "$BASE_URL$photo" else photo
                }
                mangaObj["description"]?.jsonPrimitive?.content?.let { description = it }
                mangaObj["status"]?.jsonPrimitive?.content?.uppercase()?.let { status = it }

                isAdult = mangaObj["is_adult"]?.jsonPrimitive?.let { it.content == "1" || it.content.equals("true", ignoreCase = true) } ?: false
                    || mangaObj["content_rating"]?.jsonPrimitive?.content?.lowercase() in listOf("pornographic", "erotica", "adult")

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
            }
        } catch (e: Exception) {
            System.err.println("Error parsing MangaDot details for $id: ${e.message}")
        }

        val chaptersList = mutableListOf<ChapterDto>()
        try {
            val chaptersBody = executeGet("$BASE_URL/api/manga/$id/chapters/list")
            if (chaptersBody.isNotBlank()) {
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
        if (id.isEmpty()) return emptyList()

        val body = executeGet("$BASE_URL/api/uploads/$id/images")
        if (body.isBlank()) return emptyList()

        return try {
            val root = json.parseToJsonElement(body).jsonObject
            val imagesArray = root["images"]?.jsonArray ?: return emptyList()

            val pagesList = mutableListOf<Pair<Int, PageDto>>()
            for (img in imagesArray) {
                val imgObj = img.jsonObject
                val pageIndex = imgObj["page_index"]?.jsonPrimitive?.content?.toIntOrNull() ?: (pagesList.size + 1)
                val path = imgObj["url"]?.jsonPrimitive?.content ?: imgObj["path"]?.jsonPrimitive?.content ?: continue
                val fullUrl = if (path.startsWith("/")) "$BASE_URL$path" else path

                pagesList.add(
                    pageIndex to PageDto(
                        source = SOURCE_ID,
                        url = fullUrl,
                    )
                )
            }
            pagesList.sortedBy { it.first }.map { it.second }
        } catch (e: Exception) {
            System.err.println("Error parsing MangaDot pages for chapter $id: ${e.message}")
            emptyList()
        }
    }
}
