package io.github.bakeneko.daemon

import kotlinx.serialization.Serializable
import org.koitharu.kotatsu.parsers.model.ContentRating
import org.koitharu.kotatsu.parsers.model.Manga
import org.koitharu.kotatsu.parsers.model.MangaChapter
import org.koitharu.kotatsu.parsers.model.MangaPage
import org.koitharu.kotatsu.parsers.model.MangaParserSource
import org.koitharu.kotatsu.parsers.model.MangaState
import org.koitharu.kotatsu.parsers.model.MangaTag

/**
 * DTOs serializables que viajan por el socket. Llevan los datos de
 * presentación (title, coverUrl, número de capítulo...) más la identidad
 * mínima (source, url) necesaria para reconstruir el modelo del parser
 * en el daemon cuando la app hace el round-trip del blob.
 */

@Serializable
data class SourceDto(val id: String, val name: String)

@Serializable
data class MangaDto(
    val source: String,
    val url: String,
    val title: String,
    val publicUrl: String? = null,
    val rating: Float = 0f,
    val isNsfw: Boolean = false,
    val coverUrl: String? = null,
    val largeCoverUrl: String? = null,
    val description: String? = null,
    val authors: List<String> = emptyList(),
    val state: String? = null,
    val chapters: List<ChapterDto> = emptyList(),
)

@Serializable
data class ChapterDto(
    val source: String,
    val url: String,
    val title: String,
    val number: Float = 0f,
    val volume: Int = 0,
    val scanlator: String? = null,
    val uploadDate: Long = 0L,
    val branch: String? = null,
)

@Serializable
data class PageDto(
    val source: String,
    val url: String,
    val preview: String? = null,
)

private fun parseState(name: String?): MangaState? = name?.let {
    runCatching { MangaState.valueOf(it) }.getOrNull()
}

internal fun MangaParserSource.toDto(): SourceDto = SourceDto(id = name, name = title ?: name)

internal fun Manga.toDto(): MangaDto = MangaDto(
    source = (source as? MangaParserSource)?.name ?: source.name,
    url = url,
    title = title ?: "",
    publicUrl = publicUrl,
    rating = rating,
    isNsfw = contentRating == ContentRating.ADULT,
    coverUrl = coverUrl,
    largeCoverUrl = largeCoverUrl,
    description = description,
    authors = authors.toList(),
    state = state?.name,
    chapters = chapters?.map { it.toDto() } ?: emptyList(),
)

internal fun MangaChapter.toDto(): ChapterDto = ChapterDto(
    source = (source as? MangaParserSource)?.name ?: source.name,
    url = url,
    title = title ?: "",
    number = number,
    volume = volume,
    scanlator = scanlator,
    uploadDate = uploadDate,
    branch = branch,
)

internal fun MangaPage.toDto(): PageDto = PageDto(
    source = (source as? MangaParserSource)?.name ?: source.name,
    url = url,
    preview = preview,
)

@Suppress("DEPRECATION")
/** Reconstruye un Manga mínimo (lo que el parser usa: url + source). */
internal fun MangaDto.toModel(): Manga {
    val src = MangaParserSource.entries.firstOrNull { it.name == source }
        ?: throw IllegalArgumentException("unknown source: $source")
    // Constructor con isNsfw (aceuta state y altTitle nullables — lo más
    // tolerante a versiones). El deprecation es cosmético, no rompe nada.
    return Manga(
        id = 0L,
        title = title,
        altTitle = null,
        url = url,
        publicUrl = publicUrl ?: url,
        rating = rating,
        isNsfw = isNsfw,
        coverUrl = coverUrl ?: "",
        tags = emptySet<MangaTag>(),
        state = parseState(state),
        author = null,
        largeCoverUrl = largeCoverUrl ?: "",
        description = description ?: "",
        chapters = emptyList(),
        source = src,
    )
}

internal fun ChapterDto.toModel(): MangaChapter {
    val src = MangaParserSource.entries.firstOrNull { it.name == source }
        ?: throw IllegalArgumentException("unknown source: $source")
    return MangaChapter(
        id = 0L,
        title = title,
        number = number,
        volume = volume,
        url = url,
        scanlator = scanlator,
        uploadDate = uploadDate,
        branch = branch,
        source = src,
    )
}

internal fun PageDto.toModel(): MangaPage {
    val src = MangaParserSource.entries.firstOrNull { it.name == source }
        ?: throw IllegalArgumentException("unknown source: $source")
    return MangaPage(id = 0L, url = url, preview = preview, source = src)
}