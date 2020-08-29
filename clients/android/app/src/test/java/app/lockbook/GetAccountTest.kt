package app.lockbook

import app.lockbook.core.getAccount
import app.lockbook.utils.*
import com.beust.klaxon.Klaxon
import com.github.michaelbull.result.Result
import org.junit.After
import org.junit.BeforeClass
import org.junit.Test

class GetAccountTest {
    var path = createRandomPath()

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadLib() {
            System.loadLibrary("lockbook_core")
        }
    }

    @After
    fun createDirectory() {
        path = createRandomPath()
    }

    @Test
    fun getAccountOk() {
        val coreModel = CoreModel(Config(path))
        CoreModel.generateAccount(Config(path), generateAlphaString()).component1()!!
        coreModel.getAccount().component1()!!
    }

    @Test
    fun getAccountNoAccount() {
        val coreModel = CoreModel(Config(path))
        val getAccountError = coreModel.getAccount().component2()!!
        require(getAccountError is GetAccountError.NoAccount)
    }

    @Test
    fun getAccountUnexpectedError() {
        val getAccountResult: Result<Account, GetAccountError>? =
            Klaxon().converter(getAccountConverter).parse(getAccount(""))
        val getAccountError = getAccountResult!!.component2()!!
        require(getAccountError is GetAccountError.UnexpectedError)
    }
}