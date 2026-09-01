package io.github.bakeneko.daemon

import okhttp3.Cookie
import okhttp3.CookieJar
import okhttp3.HttpUrl
import okhttp3.OkHttpClient
import okhttp3.Response
import io.github.landwarderer.futon.parsers.MangaLoaderContext
import io.github.landwarderer.futon.parsers.MangaParser
import io.github.landwarderer.futon.parsers.bitmap.Bitmap
import io.github.landwarderer.futon.parsers.config.MangaSourceConfig
import io.github.landwarderer.futon.parsers.model.MangaSource
import io.github.landwarderer.futon.parsers.network.UserAgents
import java.util.Base64
import java.util.Locale
import javax.script.ScriptEngineManager

/**
 * Adaptador de [MangaLoaderContext] para JVM headless.
 * Puerto directo del DesktopMangaLoaderContext del proyecto original.
 * Sin WebView ni browser actions (las fuentes que los necesitan fallarán
 * limpiamente con un error que la app puede mostrar/ofrecer reintentar).
 */
class DaemonLoaderContext : MangaLoaderContext() {

    override val cookieJar = object : CookieJar {
        private val cookies = mutableMapOf<String, List<Cookie>>()
        override fun saveFromResponse(url: HttpUrl, newCookies: List<Cookie>) {
            cookies[url.host] = newCookies
        }

        override fun loadForRequest(url: HttpUrl): List<Cookie> =
            cookies[url.host] ?: emptyList()
    }

    private val scriptEngine = ScriptEngineManager().getEngineByName("nashorn")

    override val httpClient: OkHttpClient = OkHttpClient.Builder()
        .cookieJar(cookieJar)
        .build()

    override suspend fun evaluateJs(baseUrl: String, script: String): String? =
        try {
            scriptEngine?.eval(script)?.toString()
        } catch (e: Exception) {
            System.err.println("evaluateJs: ${e::class.simpleName}: ${e.message}")
            null
        }

    override suspend fun evaluateJs(script: String): String? = evaluateJs("", script)

    override fun getDefaultUserAgent(): String = UserAgents.FIREFOX_DESKTOP

    override fun createBitmap(width: Int, height: Int): Bitmap = object : Bitmap {
        override val width = width
        override val height = height
        override fun drawBitmap(
            sourceBitmap: Bitmap,
            src: io.github.landwarderer.futon.parsers.bitmap.Rect,
            dst: io.github.landwarderer.futon.parsers.bitmap.Rect,
        ) {}
    }

    override fun getConfig(source: MangaSource): MangaSourceConfig = object : MangaSourceConfig {
        @Suppress("UNCHECKED_CAST")
        override fun <T> get(key: io.github.landwarderer.futon.parsers.config.ConfigKey<T>): T = key.defaultValue
    }

    override fun encodeBase64(data: ByteArray): String =
        Base64.getEncoder().encodeToString(data)

    override fun decodeBase64(data: String): ByteArray =
        Base64.getDecoder().decode(data)

    override fun getPreferredLocales(): List<Locale> = listOf(Locale.getDefault())

    override fun requestBrowserAction(parser: MangaParser, url: String): Nothing =
        throw UnsupportedOperationException("browser action required for: $url")

    override fun redrawImageResponse(response: Response, redraw: (image: Bitmap) -> Bitmap): Response = response

}
