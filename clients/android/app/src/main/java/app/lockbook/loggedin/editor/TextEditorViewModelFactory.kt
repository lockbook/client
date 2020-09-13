package app.lockbook.loggedin.editor

import android.app.Application
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider

class TextEditorViewModelFactory(
    private val application: Application,
    private val id: String,
    private val initialContents: String
) : ViewModelProvider.Factory {
    @Suppress("unchecked_cast")
    override fun <T : ViewModel?> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(TextEditorViewModel::class.java))
            return TextEditorViewModel(application, id, initialContents) as T
        throw IllegalArgumentException("Unknown ViewModel class")
    }
}