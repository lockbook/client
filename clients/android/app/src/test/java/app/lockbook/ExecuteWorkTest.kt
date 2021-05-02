package app.lockbook

import app.lockbook.core.executeWork
import app.lockbook.model.CoreModel
import app.lockbook.util.*
import com.beust.klaxon.Klaxon
import com.github.michaelbull.result.Result
import org.junit.After
import org.junit.BeforeClass
import org.junit.Test

class ExecuteWorkTest {
    var config = Config(createRandomPath())

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadLib() {
            System.loadLibrary("lockbook_core")
        }
    }

    @After
    fun createDirectory() {
        config = Config(createRandomPath())
    }

    @Test
    fun executeWorkOk() {
        assertType<Unit>(
            CoreModel.generateAccount(config, generateAlphaString()).component1()
        )

        val rootFileMetadata = assertTypeReturn<FileMetadata>(
            CoreModel.getRoot(config).component1()
        )

        val document = assertTypeReturn<FileMetadata>(
            CoreModel.createFile(
                config,
                rootFileMetadata.id,
                generateAlphaString(),
                Klaxon().toJsonString(FileType.Document)
            ).component1()
        )

        val folder = assertTypeReturn<FileMetadata>(
            CoreModel.createFile(
                config,
                rootFileMetadata.id,
                generateAlphaString(),
                Klaxon().toJsonString(FileType.Folder)
            ).component1()
        )

        assertType<Unit>(
            CoreModel.insertFile(config, document).component1()
        )

        assertType<Unit>(
            CoreModel.insertFile(config, folder).component1()
        )
        repeat(10) {
            val syncWork = assertTypeReturn<WorkCalculated>(
                CoreModel.calculateWork(config).component1()
            )

            for (workUnit in syncWork.workUnits) {
                assertType<Unit>(
                    CoreModel.executeWork(
                        config,
                        assertTypeReturn(
                            CoreModel.getAccount(config).component1()
                        ),
                        workUnit
                    ).component1()
                )
            }
        }
    }

    @Test
    fun executeWorkImportAccountOk() {
        assertType<Unit>(
            CoreModel.generateAccount(config, generateAlphaString()).component1()
        )

        val rootFileMetadata = assertTypeReturn<FileMetadata>(
            CoreModel.getRoot(config).component1()
        )

        val document = assertTypeReturn<FileMetadata>(
            CoreModel.createFile(
                config,
                rootFileMetadata.id,
                generateAlphaString(),
                Klaxon().toJsonString(FileType.Document)
            ).component1()
        )

        val folder = assertTypeReturn<FileMetadata>(
            CoreModel.createFile(
                config,
                rootFileMetadata.id,
                generateAlphaString(),
                Klaxon().toJsonString(FileType.Folder)
            ).component1()
        )

        assertType<Unit>(
            CoreModel.insertFile(config, document).component1()
        )

        assertType<Unit>(
            CoreModel.insertFile(config, folder).component1()
        )

        assertType<Unit>(
            CoreModel.sync(config, null).component1()
        )

        val exportAccountString = assertTypeReturn<String>(
            CoreModel.exportAccount(config).component1()
        )

        config = Config(createRandomPath())

        assertType<Unit>(
            CoreModel.importAccount(config, exportAccountString).component1()
        )

        repeat(10) {
            val syncWork = assertTypeReturn<WorkCalculated>(
                CoreModel.calculateWork(config).component1()
            )

            for (workUnit in syncWork.workUnits) {
                assertType<Unit>(
                    CoreModel.executeWork(
                        config,
                        assertTypeReturn(
                            CoreModel.getAccount(config).component1()
                        ),
                        workUnit
                    ).component1()
                )
            }
        }
    }

    @Test
    fun executeWorkUnexpectedError() {
        val executeSyncWorkResult: Result<Unit, ExecuteWorkError>? =
            Klaxon().converter(executeWorkConverter).parse(executeWork("", "", ""))

        assertType<ExecuteWorkError.Unexpected>(
            executeSyncWorkResult?.component2()
        )
    }
}
