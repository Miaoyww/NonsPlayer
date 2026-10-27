package com.miaoyww.nons

import java.io.File
import java.io.IOException
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** One file read and one latest pending request; completions are delivered on the caller's thread. */
internal class ArtworkFileLoader(
    private val cacheDirectory: File,
    private val post: (() -> Unit) -> Unit,
    private val isLink: (File) -> Boolean,
    private val read: (String) -> ByteArray = { path -> File(path).readBytes() }
) : AutoCloseable {
    private val revision = AtomicLong()
    private val closed = AtomicBoolean()
    private val worker = ThreadPoolExecutor(
        1, 1, 0L, TimeUnit.MILLISECONDS, ArrayBlockingQueue<Runnable>(1),
        { task -> Thread(task, "nons-media-artwork") }, ThreadPoolExecutor.DiscardOldestPolicy()
    )

    fun load(path: String, loaded: (ByteArray) -> Unit) {
        if (closed.get()) return
        val expected = revision.incrementAndGet()
        try {
            worker.execute {
                if (closed.get() || expected != revision.get()) return@execute
                val bytes = try { read(checkedPath(path)) } catch (_: IOException) { return@execute }
                    catch (_: SecurityException) { return@execute }
                if (closed.get() || expected != revision.get()) return@execute
                post {
                    if (!closed.get() && expected == revision.get()) loaded(bytes)
                }
            }
        } catch (_: RejectedExecutionException) { /* Released players cannot publish artwork. */ }
    }

    internal fun checkedPath(path: String): String {
        val root = cacheDirectory.canonicalFile
        val file = File(path)
        if (!file.isAbsolute) throw IOException("Artwork path must be absolute")
        val absolute = file.absoluteFile
        val canonical = file.canonicalFile
        val directories = listOf(File(root, "covers"), File(root, "online-covers-v1/images"))
        if (absolute != canonical || directories.none { directory ->
            directory.absoluteFile == directory.canonicalFile &&
                canonical.path.startsWith(directory.path + File.separator)
        } || !canonical.isFile) throw IOException("Artwork must be an application cover asset")
        var component = absolute
        while (component != root) {
            if (isLink(component)) throw IOException("Artwork paths cannot contain links")
            component = component.parentFile ?: throw IOException("Artwork is outside application cache")
        }
        return canonical.path
    }

    fun invalidate() {
        revision.incrementAndGet()
        worker.queue.clear()
    }

    override fun close() {
        closed.set(true)
        invalidate()
        worker.shutdownNow()
    }
}
