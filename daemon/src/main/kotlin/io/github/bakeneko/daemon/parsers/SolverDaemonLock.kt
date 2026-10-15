package io.github.bakeneko.daemon.parsers

import java.io.File
import java.io.RandomAccessFile

/**
 * Serializa el arranque del solver dentro del daemon Java y entre procesos.
 * MangaDot y ZonaTMO usan el mismo socket y el mismo perfil WebKit; levantar
 * dos instancias a la vez puede hacer que WebKit termine su NetworkProcess.
 */
internal object SolverDaemonLock {
    private val monitor = Any()

    fun <T> withLock(socketPath: String, action: () -> T): T = synchronized(monitor) {
        val lockFile = File("$socketPath.lock")
        lockFile.parentFile?.mkdirs()
        RandomAccessFile(lockFile, "rw").channel.use { channel ->
            channel.lock().use { action() }
        }
    }
}
