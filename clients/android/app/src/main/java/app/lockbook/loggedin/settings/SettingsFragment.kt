package app.lockbook.loggedin.settings

import android.content.*
import android.os.Bundle
import android.view.Gravity
import android.widget.PopupWindow
import android.widget.Toast
import androidx.biometric.BiometricConstants
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.preference.*
import app.lockbook.R
import app.lockbook.utils.AccountExportError
import app.lockbook.utils.Config
import app.lockbook.utils.CoreModel
import app.lockbook.utils.SharedPreferences.BIOMETRIC_NONE
import app.lockbook.utils.SharedPreferences.BIOMETRIC_OPTION_KEY
import app.lockbook.utils.SharedPreferences.BIOMETRIC_RECOMMENDED
import app.lockbook.utils.SharedPreferences.BIOMETRIC_STRICT
import app.lockbook.utils.SharedPreferences.EXPORT_ACCOUNT_QR_KEY
import app.lockbook.utils.SharedPreferences.EXPORT_ACCOUNT_RAW_KEY
import app.lockbook.utils.UNEXPECTED_ERROR_OCCURRED
import com.github.michaelbull.result.Err
import com.github.michaelbull.result.Ok
import com.google.zxing.BarcodeFormat
import com.journeyapps.barcodescanner.BarcodeEncoder
import kotlinx.android.synthetic.main.activity_account_qr_code.view.*
import timber.log.Timber

class SettingsFragment(private val config: Config) : PreferenceFragmentCompat() {
    override fun onCreatePreferences(savedInstanceState: Bundle?, rootKey: String?) {
        setPreferencesFromResource(R.xml.settings_preference, rootKey)

        findPreference<Preference>(BIOMETRIC_OPTION_KEY)?.setOnPreferenceChangeListener { preference, newValue ->
            if (newValue is String) {
                performBiometricFlow(preference.key, newValue)
            }

            false
        }

        if (!isBiometricsOptionsAvailable()) {
            findPreference<ListPreference>(BIOMETRIC_OPTION_KEY)?.isEnabled = false
        }
    }

    override fun onPreferenceTreeClick(preference: Preference?): Boolean {
        when (preference?.key) {
            EXPORT_ACCOUNT_QR_KEY, EXPORT_ACCOUNT_RAW_KEY -> performBiometricFlow(preference.key)
            else -> super.onPreferenceTreeClick(preference)
        }

        return true
    }

    private fun performBiometricFlow(key: String, newValue: String = "") {
        when (
            PreferenceManager.getDefaultSharedPreferences(
                requireContext()
            ).getString(
                BIOMETRIC_OPTION_KEY,
                BIOMETRIC_NONE
            )
        ) {
            BIOMETRIC_RECOMMENDED, BIOMETRIC_STRICT -> {
                if (BiometricManager.from(requireContext())
                    .canAuthenticate() != BiometricManager.BIOMETRIC_SUCCESS
                ) {
                    Timber.e("Biometric shared preference is strict despite no biometrics.")
                    Toast.makeText(
                        requireContext(),
                        UNEXPECTED_ERROR_OCCURRED,
                        Toast.LENGTH_LONG
                    )
                        .show()
                    return
                }

                val executor = ContextCompat.getMainExecutor(requireContext())
                val biometricPrompt = BiometricPrompt(
                    this, executor,
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationError(
                            errorCode: Int,
                            errString: CharSequence
                        ) {
                            super.onAuthenticationError(errorCode, errString)
                            when (errorCode) {
                                BiometricConstants.ERROR_HW_UNAVAILABLE, BiometricConstants.ERROR_UNABLE_TO_PROCESS, BiometricConstants.ERROR_NO_BIOMETRICS, BiometricConstants.ERROR_HW_NOT_PRESENT -> {
                                    Timber.e("Biometric authentication error: $errString")
                                    Toast.makeText(
                                        requireContext(),
                                        UNEXPECTED_ERROR_OCCURRED, Toast.LENGTH_SHORT
                                    )
                                        .show()
                                }
                                BiometricConstants.ERROR_LOCKOUT, BiometricConstants.ERROR_LOCKOUT_PERMANENT -> {
                                    Toast.makeText(
                                        requireContext(),
                                        "Too many tries, try again later!", Toast.LENGTH_SHORT
                                    )
                                        .show()
                                }
                            }
                        }

                        override fun onAuthenticationSucceeded(
                            result: BiometricPrompt.AuthenticationResult
                        ) {
                            super.onAuthenticationSucceeded(result)
                            matchKey(key, newValue)
                        }
                    }
                )

                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle("Lockbook Biometric Verification")
                    .setSubtitle("Enter your fingerprint to modify this biometric sensitive setting.")
                    .setDeviceCredentialAllowed(true)
                    .build()

                biometricPrompt.authenticate(promptInfo)
            }
            BIOMETRIC_NONE -> matchKey(key, newValue)
        }
    }

    private fun matchKey(key: String, newValue: String) {
        when (key) {
            EXPORT_ACCOUNT_RAW_KEY -> exportAccountRaw()
            EXPORT_ACCOUNT_QR_KEY -> exportAccountQR()
            BIOMETRIC_OPTION_KEY -> changeBiometricPreference(newValue)
        }
    }

    private fun changeBiometricPreference(newValue: String) {
        findPreference<ListPreference>(BIOMETRIC_OPTION_KEY)?.value = newValue
    }

    private fun exportAccountQR() {
        when (val exportResult = CoreModel.exportAccount(config)) {
            is Ok -> {
                val bitmap = BarcodeEncoder().encodeBitmap(
                    exportResult.value,
                    BarcodeFormat.QR_CODE,
                    400,
                    400
                )

                val qrCodeView = layoutInflater.inflate(R.layout.activity_account_qr_code, null)
                qrCodeView.qr_code.setImageBitmap(bitmap)
                val popUpWindow = PopupWindow(qrCodeView, 900, 900, true)
                popUpWindow.showAtLocation(view, Gravity.CENTER, 0, 0)
            }
            is Err -> {
                when (val error = exportResult.error) {
                    is AccountExportError.NoAccount -> Toast.makeText(
                        context,
                        "Error! No account!",
                        Toast.LENGTH_LONG
                    ).show()
                    is AccountExportError.UnexpectedError -> {
                        Timber.e("Unable to export account: ${error.error}")
                        Toast.makeText(
                            context,
                            UNEXPECTED_ERROR_OCCURRED,
                            Toast.LENGTH_LONG
                        ).show()
                    }
                }
            }
        }
    }

    private fun exportAccountRaw() {
        when (val exportResult = CoreModel.exportAccount(config)) {
            is Ok -> {
                val clipBoard: ClipboardManager =
                    requireContext().getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                val clipBoardData = ClipData.newPlainText("account string", exportResult.value)
                clipBoard.setPrimaryClip(clipBoardData)
                Toast.makeText(context, "Account string copied!", Toast.LENGTH_LONG)
                    .show()
            }
            is Err -> when (val error = exportResult.error) {
                is AccountExportError.NoAccount -> Toast.makeText(
                    context,
                    "Error! No account!",
                    Toast.LENGTH_LONG
                ).show()
                is AccountExportError.UnexpectedError -> {
                    Timber.e("Unable to export account: ${error.error}")
                    Toast.makeText(
                        context,
                        UNEXPECTED_ERROR_OCCURRED,
                        Toast.LENGTH_LONG
                    ).show()
                }
            }
        }
    }

    private fun isBiometricsOptionsAvailable(): Boolean =
        BiometricManager.from(requireContext())
            .canAuthenticate() == BiometricManager.BIOMETRIC_SUCCESS
}
