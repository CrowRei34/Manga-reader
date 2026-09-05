package io.github.bakeneko.daemon.parsers

import io.github.bakeneko.daemon.ChapterDto
import io.github.bakeneko.daemon.MangaDto
import io.github.bakeneko.daemon.PageDto
import io.github.bakeneko.daemon.SourceDto
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import okhttp3.Headers
import okhttp3.OkHttpClient
import okhttp3.Request
import org.jsoup.Jsoup
import java.net.URLEncoder
import java.text.SimpleDateFormat
import java.util.Locale

class ZonaTmo(private val httpClient: OkHttpClient) {

    companion object {
        const val SOURCE_ID = "TUMANGAONLINE"
        const val SOURCE_NAME = "TuMangaOnline"
        const val BASE_URL = "https://zonatmo.org"
        const val USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64; rv:134.0) Gecko/20100101 Firefox/134.0"

        val SOURCE_DTO = SourceDto(
            id = SOURCE_ID,
            name = SOURCE_NAME,
            language = "es",
        )
    }

    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    private fun getSavedCookies(): String? {
        val dataDir = System.getenv("XDG_DATA_HOME") ?: "${System.getenv("HOME")}/.local/share"
        val soupFile = java.io.File("$dataDir/bakeneko/solver_profile/cookies")
        if (soupFile.exists()) {
            val cookies = mutableListOf<String>()
            soupFile.forEachLine { line ->
                if (!line.startsWith("#") || line.startsWith("#HttpOnly_")) {
                    val parts = line.split("\t")
                    if (parts.size >= 7 && parts[0].contains("zonatmo.org")) {
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
                        okhttp3.Cookie.Builder()
                            .domain("zonatmo.org")
                            .path("/")
                            .name(name)
                            .value(value)
                            .build()
                            .let { list.add(it) }
                    }
                }
                return list
            }
        })
        .followRedirects(true)
        .followSslRedirects(true)
        .build()

    fun getRequestHeaders(): Headers = Headers.Builder()
        .set("User-Agent", USER_AGENT)
        .set("Referer", "$BASE_URL/")
        .set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .set("Accept-Language", "es-ES,es;q=0.9,en;q=0.8")
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
            val sockPath = getSolverSocketPath()
            val sockFile = java.io.File(sockPath)
            if (sockFile.exists()) {
                sockFile.delete()
            }

            val solverBin = System.getenv("BAKENEKO_SOLVER_PATH") ?: run {
                val execDir = try {
                    java.io.File(ZonaTmo::class.java.protectionDomain.codeSource.location.toURI()).parentFile
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
                    val projectRoot = execDir.parentFile?.parentFile?.parentFile
                    if (projectRoot != null) {
                        paths.add(java.io.File(projectRoot, "target/release/bakeneko-solver").absolutePath)
                        paths.add(java.io.File(projectRoot, "target/debug/bakeneko-solver").absolutePath)
                    }
                }
                paths.firstOrNull { java.io.File(it).exists() } ?: "bakeneko-solver"
            }
            val pb = ProcessBuilder(solverBin)
            pb.redirectOutput(ProcessBuilder.Redirect.DISCARD)
            pb.redirectError(ProcessBuilder.Redirect.DISCARD)
            pb.start()

            for (i in 0..50) {
                if (sockFile.exists()) break
                Thread.sleep(100)
            }
        } catch (e: Exception) {
            System.err.println("Error spawning bakeneko-solver daemon: ${e.message}")
        }
    }

    private fun querySolverDaemon(url: String, retryCount: Int = 0): String {
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
                if (resultStr.isNotBlank()) {
                    return resultStr
                }
            }
        } catch (e: Exception) {
            if (retryCount == 0) {
                try {
                    java.io.File(sockPath).delete()
                } catch (_: Exception) {}
                startSolverDaemon()
                return querySolverDaemon(url, retryCount + 1)
            }
            System.err.println("querySolverDaemon error: ${e.message}")
        }
        return ""
    }

    private fun executeGet(url: String): String {
        try {
            val request = Request.Builder()
                .url(url)
                .headers(getRequestHeaders())
                .get()
                .build()
            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    val body = response.body?.string() ?: ""
                    if (body.isNotBlank()) return body
                } else if (response.code == 403 || response.code == 503) {
                    System.err.println("[daemon] ZonaTMO HTTP ${response.code} (Cloudflare) on $url -> Delegating to bakeneko-solver daemon...")
                    return querySolverDaemon(url)
                }
            }
        } catch (e: Exception) {
            return querySolverDaemon(url)
        }
        return ""
    }

    private fun parseDateToMillis(dateStr: String?): Long {
        if (dateStr.isNullOrBlank()) return 0L
        val clean = dateStr.trim()
        return try {
            val sdf = SimpleDateFormat("dd/MM/yyyy", Locale.getDefault())
            sdf.parse(clean)?.time ?: 0L
        } catch (_: Exception) {
            0L
        }
    }

    suspend fun catalog(offset: Int, query: String?, categories: List<String>): List<MangaDto> {
        val hasQuery = !query.isNullOrBlank()
        if (hasQuery && (query!!.startsWith("http://") || query.startsWith("https://"))) {
            val cleanUrl = query.trim()
            val m = details(MangaDto(source = SOURCE_ID, url = cleanUrl, title = ""))
            return if (m.title.isNotBlank()) listOf(m) else emptyList()
        }

        val page = (offset / 24) + 1
        val url = if (hasQuery) {
            val enc = URLEncoder.encode(query!!.trim(), "UTF-8")
            "$BASE_URL/biblioteca?title=$enc&page=$page"
        } else {
            "$BASE_URL/biblioteca?page=$page"
        }

        val body = executeGet(url)
        if (body.isBlank()) return emptyList()

        val doc = Jsoup.parse(body, BASE_URL)
        val elements = doc.select("div.element")
        val results = mutableListOf<MangaDto>()

        for (el in elements) {
            val a = el.selectFirst("a[href]") ?: continue
            val href = a.absUrl("href")
            if (href.isBlank()) continue

            val title = el.selectFirst("h4.text-truncate")?.text()?.trim()
                ?: el.selectFirst("h4")?.text()?.trim()
                ?: a.attr("title").trim()
            if (title.isBlank()) continue

            val coverUrl = el.selectFirst("div.thumbnail[data-bg]")?.attr("data-bg")?.takeIf { it.isNotBlank() }
                ?: el.selectFirst("img.cover-bg-img")?.absUrl("src")?.takeIf { it.isNotBlank() }
                ?: el.selectFirst("img")?.absUrl("src")?.takeIf { it.isNotBlank() }

            val mature = el.selectFirst(".book-meta-mature") != null
            val scoreText = el.selectFirst("span.score span")?.text()?.trim()
            val rating = (scoreText?.toFloatOrNull() ?: 0f) / 2f
            val statusText = el.selectFirst("span.book-meta-status")?.text()?.trim()
            val state = when {
                statusText?.contains("emisión", ignoreCase = true) == true -> "ONGOING"
                statusText?.contains("finalizado", ignoreCase = true) == true -> "FINISHED"
                else -> null
            }

            results.add(
                MangaDto(
                    source = SOURCE_ID,
                    url = href,
                    title = title,
                    publicUrl = href,
                    rating = rating,
                    isNsfw = mature,
                    coverUrl = coverUrl,
                    largeCoverUrl = coverUrl,
                    description = null,
                    authors = emptyList(),
                    state = state,
                    chapters = emptyList(),
                )
            )
        }
        return results
    }

    suspend fun details(manga: MangaDto): MangaDto {
        val url = if (manga.url.startsWith("http")) manga.url else "$BASE_URL/${manga.url.removePrefix("/")}"
        val body = executeGet(url)
        if (body.isBlank()) return manga

        val doc = Jsoup.parse(body, url)

        val title = doc.selectFirst("h1.element-title")?.text()?.trim()?.takeIf { it.isNotBlank() }
            ?: doc.selectFirst("h2.element-title")?.text()?.trim()?.takeIf { it.isNotBlank() }
            ?: manga.title

        val coverUrl = doc.selectFirst("div.book-thumbnail[data-bg]")?.attr("data-bg")?.takeIf { it.isNotBlank() }
            ?: doc.selectFirst("div.book-thumbnail img")?.absUrl("src")?.takeIf { it.isNotBlank() }
            ?: doc.selectFirst("img.cover-bg-img")?.absUrl("src")?.takeIf { it.isNotBlank() }
            ?: manga.coverUrl

        val desc = doc.selectFirst("p.element-description, div.element-description")?.text()?.trim()
            ?: manga.description

        val statusStr = doc.selectFirst(".book-status, span.book-status")?.text()?.trim()
        val state = when {
            statusStr?.contains("emisión", ignoreCase = true) == true -> "ONGOING"
            statusStr?.contains("finalizado", ignoreCase = true) == true -> "FINISHED"
            else -> manga.state
        }

        val authors = doc.select(".element-staff .staff-card .staff-name")
            .map { it.text().trim() }
            .filter { it.isNotBlank() }
            .distinct()

        val isAdult = doc.selectFirst(".book-meta-mature") != null || manga.isNsfw

        val chapterElements = doc.select("ul.list-chapters li")
        val chapters = mutableListOf<ChapterDto>()

        for (li in chapterElements) {
            val numAttr = li.attr("data-chapter-number").trim()
            val numSpan = li.selectFirst(".chapter-number")?.text()?.trim() ?: ""
            val chNumber = numAttr.toFloatOrNull()
                ?: Regex("""\d+(\.\d+)?""").find(numSpan)?.value?.toFloatOrNull()
                ?: 0f

            val dateStr = li.selectFirst("span.text-muted.small")?.text()?.trim()
            val uploadDate = parseDateToMillis(dateStr)

            val uploadRows = li.select(".chapter-detail div.d-flex.align-items-center.flex-wrap")
            if (uploadRows.isNotEmpty()) {
                for (row in uploadRows) {
                    val a = row.selectFirst("a[href*=\"/view_uploads/\"]") ?: continue
                    val readLink = a.absUrl("href")
                    if (readLink.isBlank()) continue

                    val scanlator = row.selectFirst("a[href*=\"/groups/\"]")?.text()?.trim()
                        ?: row.selectFirst(".badge-light")?.text()?.replace("Subido por:", "")?.trim()
                    val titleText = numSpan.ifEmpty { "Capítulo $chNumber" }

                    chapters.add(
                        ChapterDto(
                            source = SOURCE_ID,
                            url = readLink,
                            title = titleText,
                            number = chNumber,
                            volume = 0,
                            language = "es",
                            scanlator = scanlator,
                            uploadDate = uploadDate,
                            branch = scanlator,
                        )
                    )
                }
            } else {
                val a = li.selectFirst("a[href*=\"/view_uploads/\"]") ?: continue
                val readLink = a.absUrl("href")
                if (readLink.isNotBlank()) {
                    val titleText = numSpan.ifEmpty { "Capítulo $chNumber" }
                    chapters.add(
                        ChapterDto(
                            source = SOURCE_ID,
                            url = readLink,
                            title = titleText,
                            number = chNumber,
                            volume = 0,
                            language = "es",
                            scanlator = null,
                            uploadDate = uploadDate,
                            branch = null,
                        )
                    )
                }
            }
        }

        return MangaDto(
            source = SOURCE_ID,
            url = url,
            title = title,
            publicUrl = url,
            rating = manga.rating,
            isNsfw = isAdult,
            coverUrl = coverUrl,
            largeCoverUrl = coverUrl,
            description = desc,
            authors = if (authors.isNotEmpty()) authors else manga.authors,
            state = state,
            chapters = chapters,
        )
    }

    suspend fun pages(chapter: ChapterDto): List<PageDto> {
        val body = executeGet(chapter.url)
        if (body.isBlank()) return emptyList()

        val doc = Jsoup.parse(body, chapter.url)
        val pageUrls = mutableListOf<String>()
        val imgElements = doc.select("img")

        for (img in imgElements) {
            val src = img.attr("data-src").takeIf { it.isNotBlank() }
                ?: img.attr("src").takeIf { it.isNotBlank() }
                ?: continue

            if (src.contains("/chapters/") || (src.contains("storage") && !src.contains("logo") && !src.contains("cover") && !src.contains("avatar") && !src.contains("icon") && !src.contains("favicon"))) {
                val absSrc = if (src.startsWith("//")) "https:$src" else if (src.startsWith("http")) src else "$BASE_URL/${src.removePrefix("/")}"
                if (!pageUrls.contains(absSrc)) {
                    pageUrls.add(absSrc)
                }
            }
        }

        return pageUrls.map { PageDto(source = SOURCE_ID, url = it) }
    }
}
