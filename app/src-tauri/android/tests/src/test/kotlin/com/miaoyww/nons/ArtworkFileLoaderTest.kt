package com.miaoyww.nons

import java.io.File
import java.io.IOException
import java.util.Collections
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Test
import org.junit.Rule
import org.junit.Assume.assumeNoException
import org.junit.rules.TemporaryFolder

class ArtworkFileLoaderTest {
    @get:Rule val temporary = TemporaryFolder()
    private val isLink: (File) -> Boolean = { java.nio.file.Files.isSymbolicLink(it.toPath()) }
    private fun asset(name: String): String = File(temporary.root, "covers/$name").apply {
        requireNotNull(parentFile).mkdirs()
        writeBytes(byteArrayOf(0))
    }.canonicalPath
    private fun LinkedBlockingQueue<() -> Unit>.completion(): () -> Unit =
        poll(5, TimeUnit.SECONDS) ?: error("Artwork read did not complete")

    @Test
    fun reads_large_file_in_background_and_posts_unchanged_bytes() {
        val file = File(asset("large.img"))
        val bytes = ByteArray(9 * 1024 * 1024) { (it % 251).toByte() }
        file.writeBytes(bytes)
        val caller = Thread.currentThread()
        val reader = AtomicReference<Thread>()
        val posted = LinkedBlockingQueue<() -> Unit>()
        val loader = ArtworkFileLoader(temporary.root, { posted.put(it) }, isLink, { path ->
            reader.set(Thread.currentThread())
            File(path).readBytes()
        })
        try {
            var delivered: ByteArray? = null
            loader.load(file.path) { delivered = it }
            val complete = posted.completion()
            assertNotSame(caller, reader.get())
            assertNull(delivered)
            complete()
            assertArrayEquals(bytes, delivered)
        } finally { loader.close(); file.delete() }
    }

    @Test
    fun invalidation_and_release_discard_already_posted_results() {
        val posted = LinkedBlockingQueue<() -> Unit>()
        val first = asset("first")
        val secondPath = asset("second")
        val loader = ArtworkFileLoader(temporary.root, { posted.put(it) }, isLink, { byteArrayOf(1) })
        try {
            var delivered = false
            loader.load(first) { delivered = true }
            val old = posted.completion()
            loader.invalidate()
            old()
            assertFalse(delivered)
            loader.load(secondPath) { delivered = true }
            val second = posted.completion()
            loader.close()
            second()
            assertFalse(delivered)
            loader.load("released") { delivered = true }
            assertTrue(posted.isEmpty())
        } finally { loader.close() }
    }

    @Test
    fun a_slow_old_read_keeps_only_the_latest_queued_request() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val read = Collections.synchronizedList(mutableListOf<String>())
        val posted = LinkedBlockingQueue<() -> Unit>()
        val paths = listOf("old", "superseded-one", "superseded-two", "latest").associateWith { asset(it) }
        val loader = ArtworkFileLoader(temporary.root, { posted.put(it) }, isLink, { path ->
            val name = File(path).name
            read.add(name)
            if (name == "old") {
                entered.countDown()
                check(release.await(5, TimeUnit.SECONDS))
            }
            name.toByteArray()
        })
        try {
            var delivered = ""
            loader.load(paths.getValue("old")) { delivered = String(it) }
            assertTrue(entered.await(5, TimeUnit.SECONDS))
            loader.load(paths.getValue("superseded-one")) { delivered = String(it) }
            loader.load(paths.getValue("superseded-two")) { delivered = String(it) }
            loader.load(paths.getValue("latest")) { delivered = String(it) }
            release.countDown()
            posted.completion()()
            assertEquals("latest", delivered)
            assertEquals(listOf("old", "latest"), read.toList())
            assertTrue(posted.isEmpty())
        } finally { release.countDown(); loader.close() }
    }

    @Test
    fun failed_file_read_does_not_prevent_the_next_image() {
        val failed = CountDownLatch(1)
        val posted = LinkedBlockingQueue<() -> Unit>()
        val missing = asset("missing")
        val valid = asset("valid")
        val loader = ArtworkFileLoader(temporary.root, { posted.put(it) }, isLink, { path ->
            if (File(path).name == "missing") { failed.countDown(); throw IOException("missing") }
            byteArrayOf(3)
        })
        try {
            var delivered: ByteArray? = null
            loader.load(missing) { delivered = it }
            assertTrue(failed.await(5, TimeUnit.SECONDS))
            loader.load(valid) { delivered = it }
            posted.completion()()
            assertArrayEquals(byteArrayOf(3), delivered)
        } finally { loader.close() }
    }

    @Test
    fun path_gate_accepts_only_ordinary_assets_in_cover_directories() {
        val local = asset("cover.img")
        val online = File(temporary.root, "online-covers-v1/images/online.jpg").apply {
            requireNotNull(parentFile).mkdirs(); writeBytes(byteArrayOf(1))
        }
        val outside = temporary.newFile("private-data")
        val loader = ArtworkFileLoader(temporary.root, { it() }, isLink)
        try {
            assertEquals(local, loader.checkedPath(local))
            assertEquals(online.canonicalPath, loader.checkedPath(online.canonicalPath))
            for (path in listOf(outside.canonicalPath, "relative.img", File(temporary.root, "covers").canonicalPath,
                File(temporary.root, "covers/../private-data").absolutePath,
                File(temporary.root, "online-covers-v1/index.sqlite3").absolutePath)) {
                try { loader.checkedPath(path); fail("Accepted $path") } catch (_: IOException) { }
            }
        } finally { loader.close() }
    }

    @Test
    fun path_gate_rejects_linked_files_and_directories_even_inside_cache() {
        val original = File(asset("original.img"))
        val linked = File(temporary.root, "covers/linked.img")
        try { java.nio.file.Files.createSymbolicLink(linked.toPath(), original.toPath()) }
        catch (error: Exception) { assumeNoException(error) }
        val directoryLink = File(temporary.root, "covers/linked-dir")
        java.nio.file.Files.createSymbolicLink(directoryLink.toPath(), requireNotNull(original.parentFile).toPath())
        val loader = ArtworkFileLoader(temporary.root, { it() }, isLink)
        try {
            for (path in listOf(linked.absolutePath, File(directoryLink, original.name).absolutePath)) {
                try { loader.checkedPath(path); fail("Accepted a directory or file link") } catch (_: IOException) { }
            }
        } finally { loader.close() }
    }
}
