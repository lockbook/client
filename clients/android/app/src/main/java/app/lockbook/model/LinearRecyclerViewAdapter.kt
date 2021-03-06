package app.lockbook.model

import android.view.LayoutInflater
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.cardview.widget.CardView
import androidx.core.content.res.ResourcesCompat
import app.lockbook.R
import app.lockbook.util.*

class LinearRecyclerViewAdapter(listFilesClickInterface: ListFilesClickInterface, override var selectedFiles: MutableList<ClientFileMetadata> = mutableListOf()) :
    GeneralViewAdapter(listFilesClickInterface) {

    override var files = listOf<ClientFileMetadata>()
        set(value) {
            field = value
            notifyDataSetChanged()
        }

    override var selectionMode = false

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): FileViewHolder =
        FileViewHolder(
            LayoutInflater.from(parent.context)
                .inflate(R.layout.linear_layout_file_item, parent, false) as CardView
        )

    override fun getItemCount(): Int = files.size

    override fun onBindViewHolder(holder: FileViewHolder, position: Int) {
        val item = files[position]

        holder.fileMetadata = item
        holder.cardView.findViewById<TextView>(R.id.linear_file_name).text = item.name
        holder.cardView.findViewById<TextView>(R.id.linear_file_description).text = holder.cardView.resources.getString(
            R.string.last_synced,
            CoreModel.convertToHumanDuration(item.metadataVersion)
        )

        val resources = holder.cardView.resources
        val theme = holder.cardView.context.theme

        val fileIcon = holder.cardView.findViewById<ImageView>(R.id.linear_file_icon)
        when {
            selectedFiles.contains(item) -> {
                holder.cardView.background.setTint(
                    ResourcesCompat.getColor(
                        resources,
                        R.color.selectedFileBackground,
                        theme
                    )
                )
                fileIcon.setImageResource(R.drawable.ic_baseline_check_24)
            }
            item.fileType == FileType.Document && item.name.endsWith(".draw") -> {
                holder.cardView.background.setTint(
                    ResourcesCompat.getColor(
                        resources,
                        R.color.colorPrimaryDark,
                        theme
                    )
                )
                fileIcon.setImageResource(R.drawable.ic_baseline_border_color_24)
            }
            item.fileType == FileType.Document -> {
                holder.cardView.background.setTint(
                    ResourcesCompat.getColor(
                        resources,
                        R.color.colorPrimaryDark,
                        theme
                    )
                )
                fileIcon.setImageResource(R.drawable.ic_baseline_insert_drive_file_24)
            }
            else -> {
                holder.cardView.background.setTint(
                    ResourcesCompat.getColor(
                        resources,
                        R.color.colorPrimaryDark,
                        theme
                    )
                )
                fileIcon.setImageResource(R.drawable.round_folder_white_18dp)
            }
        }
    }
}
