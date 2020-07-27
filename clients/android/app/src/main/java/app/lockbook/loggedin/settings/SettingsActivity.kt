package app.lockbook.loggedin.settings

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.widget.PopupWindow
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.biometric.BiometricConstants
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.databinding.DataBindingUtil
import androidx.lifecycle.Observer
import androidx.lifecycle.ViewModelProvider
import androidx.recyclerview.widget.LinearLayoutManager
import app.lockbook.R
import app.lockbook.databinding.ActivitySettingsBinding
import app.lockbook.loggedin.mainscreen.MainScreenActivity
import app.lockbook.utils.SharedPreferences
import app.lockbook.utils.SharedPreferences.BIOMETRIC_NONE
import kotlinx.android.synthetic.main.activity_account_qr_code.*
import kotlinx.android.synthetic.main.activity_account_qr_code.view.*
import kotlinx.android.synthetic.main.activity_settings.*

class SettingsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val binding: ActivitySettingsBinding = DataBindingUtil.setContentView(
            this,
            R.layout.activity_settings
        )

        val settings = resources.getStringArray(R.array.settings_names).toList()
        val settingsViewModelFactory =
            SettingsViewModelFactory(settings, application.filesDir.absolutePath)
        val settingsViewModel =
            ViewModelProvider(this, settingsViewModelFactory).get(SettingsViewModel::class.java)
        val adapter = SettingsAdapter(settings, settingsViewModel)

        binding.settingsViewModel = settingsViewModel
        binding.settingsList.adapter = adapter
        binding.settingsList.layoutManager = LinearLayoutManager(applicationContext)
        binding.lifecycleOwner = this

        settingsViewModel.errorHasOccurred.observe(this, Observer { errorText ->
            errorHasOccurred(errorText)
        })

        settingsViewModel.navigateToAccountQRCode.observe(this, Observer { qrBitmap ->
            if(getSharedPreferences(SharedPreferences.SHARED_PREF_FILE, Context.MODE_PRIVATE).getInt(SharedPreferences.BIOMETRIC_OPTION_KEY, BIOMETRIC_NONE) != BIOMETRIC_NONE) {
                if (BiometricManager.from(applicationContext)
                        .canAuthenticate() != BiometricManager.BIOMETRIC_SUCCESS
                ) {
                    Toast.makeText(this, "An unexpected error has occurred!", Toast.LENGTH_LONG)
                        .show()
                    finish()
                }

                val executor = ContextCompat.getMainExecutor(this)
                val biometricPrompt = BiometricPrompt(this, executor,
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationError(
                            errorCode: Int,
                            errString: CharSequence
                        ) {
                            super.onAuthenticationError(errorCode, errString)
                            when(errorCode) {
                                BiometricConstants.ERROR_HW_UNAVAILABLE, BiometricConstants.ERROR_UNABLE_TO_PROCESS, BiometricConstants.ERROR_NO_BIOMETRICS, BiometricConstants.ERROR_HW_NOT_PRESENT -> {
                                    Log.i("Launch", "Biometric authentication error: $errString")
                                    Toast.makeText(
                                        applicationContext,
                                        "An unexpected error has occurred!", Toast.LENGTH_SHORT
                                    )
                                        .show()
                                }
                                else -> {}
                            }
                        }

                        override fun onAuthenticationSucceeded(
                            result: BiometricPrompt.AuthenticationResult
                        ) {
                            super.onAuthenticationSucceeded(result)
                            navigateToAccountQRCode(qrBitmap)
                        }

                        override fun onAuthenticationFailed() {
                            super.onAuthenticationFailed()
                            Toast.makeText(
                                applicationContext,
                                "Invalid fingerprint.", Toast.LENGTH_SHORT
                            )
                                .show()
                        }
                    })

                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle("Lockbook Biometric Verification")
                    .setSubtitle("Login to view your account string.")
                    .setNegativeButtonText("Cancel")
                    .build()

                biometricPrompt.authenticate(promptInfo)
            }
        })

        settingsViewModel.copyAccountString.observe(this, Observer {accountString ->
            if(getSharedPreferences(SharedPreferences.SHARED_PREF_FILE, Context.MODE_PRIVATE).getInt(SharedPreferences.BIOMETRIC_OPTION_KEY, BIOMETRIC_NONE) != BIOMETRIC_NONE) {
                if (BiometricManager.from(applicationContext)
                        .canAuthenticate() != BiometricManager.BIOMETRIC_SUCCESS
                ) {
                    Toast.makeText(this, "An unexpected error has occurred!", Toast.LENGTH_LONG)
                        .show()
                    finish()
                }

                val executor = ContextCompat.getMainExecutor(this)
                val biometricPrompt = BiometricPrompt(this, executor,
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationError(
                            errorCode: Int,
                            errString: CharSequence
                        ) {
                            super.onAuthenticationError(errorCode, errString)
                            Log.i("Launch", "Biometric authentication error: $errString")
                            when(errorCode) {
                                BiometricConstants.ERROR_HW_UNAVAILABLE, BiometricConstants.ERROR_UNABLE_TO_PROCESS, BiometricConstants.ERROR_NO_BIOMETRICS, BiometricConstants.ERROR_HW_NOT_PRESENT -> {
                                    Log.i("Launch", "Biometric authentication error: $errString")
                                    Toast.makeText(
                                        applicationContext,
                                        "An unexpected error has occurred!", Toast.LENGTH_SHORT
                                    )
                                        .show()
                                }
                                else -> {}
                            }
                        }

                        override fun onAuthenticationSucceeded(
                            result: BiometricPrompt.AuthenticationResult
                        ) {
                            super.onAuthenticationSucceeded(result)
                            copyAccountString(accountString)
                        }

                        override fun onAuthenticationFailed() {
                            super.onAuthenticationFailed()
                            Toast.makeText(
                                applicationContext,
                                "Invalid fingerprint.", Toast.LENGTH_SHORT
                            )
                                .show()
                        }
                    })

                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle("Lockbook Biometric Verification")
                    .setSubtitle("Login to view your account string.")
                    .setNegativeButtonText("Cancel")
                    .build()

                biometricPrompt.authenticate(promptInfo)
            }
        })
    }

    private fun copyAccountString(accountString: String) {
        val clipBoard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clipBoardData = ClipData.newPlainText("account string", accountString)
        clipBoard.setPrimaryClip(clipBoardData)
        Toast.makeText(this, "Account string copied!", Toast.LENGTH_LONG).show()
    }

    private fun navigateToAccountQRCode(qrBitmap: Bitmap) {
        val qrCodeView = layoutInflater.inflate(R.layout.activity_account_qr_code, null)
        qrCodeView.qr_code.setImageBitmap(qrBitmap)
        val popUpWindow = PopupWindow(qrCodeView, 900, 900, true)
        popUpWindow.showAtLocation(settings_linear_layout, Gravity.CENTER, 0, 0)
    }

    private fun errorHasOccurred(errorText: String) {
        Toast.makeText(this, errorText, Toast.LENGTH_LONG).show()
    }

}
