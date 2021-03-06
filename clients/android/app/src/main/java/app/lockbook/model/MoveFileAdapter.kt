package app.lockbook.model

import android.view.LayoutInflater
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.cardview.widget.CardView
import androidx.recyclerview.widget.RecyclerView
import app.lockbook.R
import app.lockbook.util.ClientFileMetadata
import app.lockbook.util.RegularClickInterface
import java.sql.Date
import java.sql.Timestamp

class MoveFileAdapter(val clickInterface: RegularClickInterface) :
    RecyclerView.Adapter<MoveFileAdapter.MoveFileViewHolder>() {

    var files = listOf<ClientFileMetadata>()
        set(value) {
            field = value
            notifyDataSetChanged()
        }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): MoveFileViewHolder =
        MoveFileViewHolder(
            LayoutInflater.from(parent.context)
                .inflate(R.layout.linear_layout_file_item, parent, false) as CardView
        )

    override fun getItemCount(): Int = files.size

    override fun onBindViewHolder(holder: MoveFileViewHolder, position: Int) {
        val item = files[position]

        val date = Date(Timestamp(item.metadataVersion).time)
        holder.fileMetadata = item
        holder.cardView.findViewById<TextView>(R.id.linear_file_name).text = item.name

        holder.cardView.findViewById<TextView>(R.id.linear_file_description).text = if (position != 0) {
            holder.cardView.resources.getString(
                R.string.last_synced,
                if (item.metadataVersion != 0L) date else holder.cardView.resources.getString(R.string.never_synced)
            )
        } else {
            item.parent
        }

        holder.cardView.findViewById<ImageView>(R.id.linear_file_icon).setImageResource(R.drawable.round_folder_white_18dp)
    }

    inner class MoveFileViewHolder(val cardView: CardView) : RecyclerView.ViewHolder(cardView) {
        lateinit var fileMetadata: ClientFileMetadata

        init {
            cardView.setOnClickListener {
                clickInterface.onItemClick(adapterPosition)
            }
        }
    }
}
