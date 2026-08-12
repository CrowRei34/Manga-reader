package io.github.bakeneko.daemon

import kotlinx.serialization.json.Json
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.koitharu.kotatsu.parsers.model.MangaParserSource

class ModelsTest {

    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = false }

    @Test
    fun `source dto round trips`() {
        val dto = MangaParserSource.MANGADEX.toDto()
        val s = json.encodeToString(SourceDto.serializer(), dto)
        val back = json.decodeFromString(SourceDto.serializer(), s)
        assertEquals(dto, back)
        assertEquals("MANGADEX", back.id)
    }

    @Test
    fun `manga dto preserves identity on round trip`() {
        val dto = MangaDto(
            source = "MANGADEX",
            url = "https://mangadex.org/title/abc",
            title = "Test Manga",
            coverUrl = "https://cdn/m/abc/cover.jpg",
        )
        val s = json.encodeToString(MangaDto.serializer(), dto)
        val back = json.decodeFromString(MangaDto.serializer(), s)
        assertEquals(dto.source, back.source)
        assertEquals(dto.url, back.url)
        assertEquals(dto.title, back.title)
        assertEquals(dto.coverUrl, back.coverUrl)
    }

    @Test
    fun `dto to model reconstructs source and url`() {
        val dto = MangaDto(source = "MANGADEX", url = "https://mangadex.org/x", title = "X")
        val model = dto.toModel()
        assertEquals("https://mangadex.org/x", model.url)
        assertEquals(MangaParserSource.MANGADEX, model.source)
    }

    @Test
    fun `chapter dto round trip keeps identity`() {
        val dto = ChapterDto(source = "MANGADEX", url = "https://mangadex.org/c1", title = "Cap 1", number = 1f)
        val model = dto.toModel()
        assertEquals(dto.url, model.url)
        assertEquals(1f, model.number)
    }

    @Test
    fun `chapter list may be empty`() {
        val dto = MangaDto(source = "MANGADEX", url = "u", title = "t")
        assertTrue(dto.chapters.isEmpty())
    }
}