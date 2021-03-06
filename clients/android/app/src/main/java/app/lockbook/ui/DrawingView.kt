package app.lockbook.ui

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.*
import android.os.Build
import android.util.AttributeSet
import android.view.*
import androidx.core.content.res.ResourcesCompat
import app.lockbook.R
import app.lockbook.model.AlertModel
import app.lockbook.screen.DrawingActivity
import app.lockbook.util.*
import java.lang.ref.WeakReference
import java.util.*
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sqrt

class DrawingView(context: Context, attributeSet: AttributeSet?) :
    SurfaceView(context, attributeSet), Runnable, SurfaceHolder.Callback {
    lateinit var drawing: Drawing
    private lateinit var canvasBitmap: Bitmap
    private lateinit var tempCanvas: Canvas
    private var thread: Thread? = null
    private var isThreadAvailable = false
    private var isDrawingAvailable = false

    private var erasePoints =
        Pair(PointF(Float.NaN, Float.NaN), PointF(Float.NaN, Float.NaN)) // Shouldn't these be NAN
    private var penSizeMultiplier = 7
    private var strokeAlpha = 255
    var isErasing = false
    var strokeColor = ColorAlias.White
    lateinit var colorAliasInARGB: EnumMap<ColorAlias, Int?>

    // Current drawing stroke state
    private val strokePaint = Paint()
    private val bitmapPaint = Paint()
    private val backgroundPaint = Paint()
    private val lastPoint = PointF()
    private var rollingAveragePressure = Float.NaN
    private val strokePath = Path()
    private val strokesBounds = mutableListOf<RectF>()

    // Scaling and Viewport state
    private val viewPort = Rect()
    private var onScreenFocusPoint = PointF()
    private var modelFocusPoint = PointF()
    private var driftWhileScalingX = 0f
    private var driftWhileScalingY = 0f

    sealed class Tool {
        object Eraser : Tool()
        data class Pen(val colorAlias: ColorAlias) : Tool()
    }

    private val alertModel by lazy {
        AlertModel(WeakReference(context as DrawingActivity))
    }

    companion object {
        const val CANVAS_WIDTH = 2125
        const val CANVAS_HEIGHT = 2750

        const val PRESSURE_SAMPLES_AVERAGED = 5
        const val SPEN_ACTION_DOWN = 211
    }

    private val scaleGestureDetector =
        ScaleGestureDetector(
            context,
            object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
                override fun onScaleBegin(detector: ScaleGestureDetector?): Boolean {
                    if (detector != null) {
                        onScreenFocusPoint = PointF(detector.focusX, detector.focusY)
                        modelFocusPoint = screenToModel(onScreenFocusPoint) ?: return false
                    }
                    return true
                }

                override fun onScale(detector: ScaleGestureDetector): Boolean {
                    drawing.scale *= detector.scaleFactor

                    val screenLocationNormalized = PointF(
                        onScreenFocusPoint.x / tempCanvas.clipBounds.width(),
                        onScreenFocusPoint.y / tempCanvas.clipBounds.height()
                    )

                    val currentViewPortWidth =
                        tempCanvas.clipBounds.width() / drawing.scale
                    val currentViewPortHeight =
                        tempCanvas.clipBounds.height() / drawing.scale

                    driftWhileScalingX =
                        (onScreenFocusPoint.x - detector.focusX) / drawing.scale
                    driftWhileScalingY =
                        (onScreenFocusPoint.y - detector.focusY) / drawing.scale

                    val left =
                        ((modelFocusPoint.x + (1 - screenLocationNormalized.x) * currentViewPortWidth) - currentViewPortWidth) + driftWhileScalingX
                    val top =
                        ((modelFocusPoint.y + (1 - screenLocationNormalized.y) * currentViewPortHeight) - currentViewPortHeight) + driftWhileScalingY
                    val right = left + currentViewPortWidth
                    val bottom = top + currentViewPortHeight

                    viewPort.set(left.toInt(), top.toInt(), right.toInt(), bottom.toInt())

                    drawing.translationX = -left
                    drawing.translationY = -top

                    return true
                }

                override fun onScaleEnd(detector: ScaleGestureDetector?) {
                    driftWhileScalingX = 0f
                    driftWhileScalingY = 0f
                    super.onScaleEnd(detector)
                }
            }
        )

    init {
        holder.setKeepScreenOn(true)
        holder.addCallback(this)

        setUpPaint()
    }

    private fun setUpPaint() {
        strokePaint.isAntiAlias = true
        strokePaint.style = Paint.Style.STROKE
        strokePaint.strokeJoin = Paint.Join.ROUND
        strokePaint.color = Color.WHITE
        strokePaint.strokeCap = Paint.Cap.ROUND

        bitmapPaint.strokeCap = Paint.Cap.ROUND
        bitmapPaint.strokeJoin = Paint.Join.ROUND

        backgroundPaint.style = Paint.Style.FILL

        strokeColor = ColorAlias.White
    }

    private fun render(canvas: Canvas) {
        canvas.save()
        canvas.scale(
            drawing.scale,
            drawing.scale,
        )

        canvas.translate(
            drawing.translationX,
            drawing.translationY
        )

        backgroundPaint.color = ResourcesCompat.getColor(
            resources,
            R.color.drawingUntouchableBackground,
            context.theme
        )

        canvas.drawPaint(backgroundPaint)

        backgroundPaint.color = ResourcesCompat.getColor(
            resources,
            R.color.drawingTouchableBackground,
            context.theme
        )

        canvas.drawRect(Rect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT), backgroundPaint)
        canvas.drawBitmap(canvasBitmap, 0f, 0f, bitmapPaint)
        canvas.restore()
    }

    private fun initializeCanvasesAndBitmaps() {
        canvasBitmap = Bitmap.createBitmap(CANVAS_WIDTH, CANVAS_HEIGHT, Bitmap.Config.ARGB_8888)
        tempCanvas = Canvas(canvasBitmap)
    }

    private fun getColor(colorAlias: ColorAlias, alpha: Float): Int? {
        val alphaAsInt = (alpha * 255).toInt()

        return if (alphaAsInt == 255) {
            colorAliasInARGB[colorAlias]
        } else {
            drawing.getARGBColor(resources.configuration.uiMode, colorAlias, alphaAsInt)
        }
    }

    private fun restoreFromModel() {
        for (stroke in drawing.strokes) {
            val strokeColor = getColor(stroke.color, stroke.alpha)

            if (strokeColor == null) {
                alertModel.notifyBasicError((context as DrawingActivity)::finish)
                return
            }

            strokesBounds.add(RectF())
            strokePaint.color = strokeColor

            for (pointIndex in 0..(stroke.pointsX.size - 2)) {
                val x1 = stroke.pointsX[pointIndex]
                val y1 = stroke.pointsY[pointIndex]

                val x2 = stroke.pointsX[pointIndex + 1]
                val y2 = stroke.pointsY[pointIndex + 1]

                val pointWidth1 = stroke.pointsGirth[pointIndex]
                val pointWidth2 = stroke.pointsGirth[pointIndex + 1]
                if (pointIndex == 0) {
                    strokesBounds.last()
                        .set(x1 - pointWidth1, y1 - pointWidth1, x1 + pointWidth1, y1 + pointWidth1)
                    updateLastStrokeBounds(x2, y2, pointWidth2)
                } else {
                    updateLastStrokeBounds(x1, y1, pointWidth1)
                    updateLastStrokeBounds(x2, y2, pointWidth2)
                }

                strokePaint.strokeWidth = pointWidth1
                strokePath.moveTo(
                    x1,
                    y1
                )
                strokePath.lineTo(
                    x2,
                    y2
                )
                tempCanvas.drawPath(strokePath, strokePaint)
                strokePath.reset()
            }

            strokePath.reset()
        }

        val strokeColor = colorAliasInARGB[ColorAlias.White]

        if (strokeColor == null) {
            alertModel.notifyBasicError((context as DrawingActivity)::finish)
            return
        }

        strokePaint.color = strokeColor

        val currentViewPortWidth =
            tempCanvas.clipBounds.width() / drawing.scale
        val currentViewPortHeight =
            tempCanvas.clipBounds.height() / drawing.scale
        viewPort.left = -drawing.translationX.toInt()
        viewPort.top = -drawing.translationY.toInt()
        viewPort.right = (viewPort.left + currentViewPortWidth).toInt()
        viewPort.bottom = (viewPort.top + currentViewPortHeight).toInt()
    }

    private fun updateLastStrokeBounds(x: Float, y: Float, pointWidth: Float) {
        val currentStrokeBounds = strokesBounds.last()
        val left = x - pointWidth
        val top = y - pointWidth
        val right = x + pointWidth
        val bottom = y + pointWidth

        if (right > currentStrokeBounds.right) {
            currentStrokeBounds.right = right
        } else if (left < currentStrokeBounds.left) {
            currentStrokeBounds.left = left
        }

        if (top < currentStrokeBounds.top) {
            currentStrokeBounds.top = top
        } else if (bottom > currentStrokeBounds.bottom) {
            currentStrokeBounds.bottom = bottom
        }
    }

    private fun doesEraserSegmentIntersectStroke(
        x1: Float,
        y1: Float,
        x2: Float,
        y2: Float,
        strokeIndex: Int
    ): Boolean {
        val currentStrokeBounds = strokesBounds[strokeIndex]
        val eraseBounds = RectF()

        if (x1 > x2) {
            eraseBounds.right = x1
            eraseBounds.left = x2
        } else {
            eraseBounds.right = x2
            eraseBounds.left = x1
        }

        if (y1 > y2) {
            eraseBounds.bottom = y1
            eraseBounds.top = y2
        } else {
            eraseBounds.bottom = y2
            eraseBounds.top = y1
        }

        // expand the erasing bounds to catch the small strokes (like dots) that would not be caught otherwise
        eraseBounds.top -= 20
        eraseBounds.bottom += 20
        eraseBounds.left -= 20
        eraseBounds.right += 20

        return RectF.intersects(currentStrokeBounds, eraseBounds)
    }

    private fun screenToModel(screen: PointF): PointF? {
        var modelX =
            (viewPort.width() * (screen.x / tempCanvas.clipBounds.width())) + viewPort.left

        if (modelX < 0) modelX = 0f
        if (modelX > tempCanvas.clipBounds.width()) modelX =
            tempCanvas.clipBounds.width().toFloat()

        var modelY =
            (viewPort.height() * (screen.y / tempCanvas.clipBounds.height())) + viewPort.top
        if (modelY < 0) modelY = 0f
        if (modelY > tempCanvas.clipBounds.height()) modelY =
            tempCanvas.clipBounds.height().toFloat()

        if (modelX.isNaN() || modelY.isNaN()) {
            return null
        }

        modelX = (modelX * 100).roundToInt() / 100f
        modelY = (modelY * 100).roundToInt() / 100f

        if (modelX < 0) {
            modelX = 0f
        } else if (modelX > CANVAS_WIDTH) {
            modelX = CANVAS_WIDTH.toFloat()
        }

        if (modelY < 0) {
            modelY = 0f
        } else if (modelY > CANVAS_HEIGHT) {
            modelY = CANVAS_HEIGHT.toFloat()
        }

        return PointF(modelX, modelY)
    }

    fun initializeWithDrawing(maybeDrawing: Drawing) {
        visibility = View.VISIBLE
        this.drawing = maybeDrawing

        initializeCanvasesAndBitmaps()
        restoreFromModel()

        isDrawingAvailable = true
        if (isThreadAvailable) {
            startThread()
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent?): Boolean {
        if (event != null) {
            val toolType = event.getToolType(0)

            if (toolType == MotionEvent.TOOL_TYPE_STYLUS ||
                toolType == MotionEvent.TOOL_TYPE_ERASER
            ) {
                handleStylusEvent(event)
            }
            if (toolType == MotionEvent.TOOL_TYPE_FINGER) {
                handleFingerEvent(event)
            }
        }

        return true
    }

    private fun handleFingerEvent(event: MotionEvent) {
        scaleGestureDetector.onTouchEvent(event)
    }

    private fun handleStylusEvent(event: MotionEvent) {
        val modelPoint = screenToModel(PointF(event.x, event.y)) ?: return
        val action = event.action

        if (action == SPEN_ACTION_DOWN) { // stay erasing if the button isn't held but it is the same stroke && vice versa
            isErasing = true
        } else if (isErasing && action == MotionEvent.ACTION_DOWN) {
            isErasing = false
        }

        if (isErasing) {
            if ((action == SPEN_ACTION_DOWN || action == MotionEvent.ACTION_DOWN) && (!erasePoints.first.x.isNaN() || !erasePoints.second.x.isNaN())) {
                erasePoints.first.set(PointF(Float.NaN, Float.NaN))
                erasePoints.second.set(PointF(Float.NaN, Float.NaN))
            }

            eraseAtPoint(modelPoint)
        } else {
            when (action) {
                MotionEvent.ACTION_DOWN -> moveTo(modelPoint, event.pressure)
                MotionEvent.ACTION_MOVE -> lineTo(modelPoint, event.pressure)
            }
        }
    }

    private fun getAdjustedPressure(pressure: Float): Float =
        ((pressure * penSizeMultiplier) * 100).roundToInt() / 100f

    private fun moveTo(point: PointF, pressure: Float) {
        lastPoint.set(point)
        rollingAveragePressure = getAdjustedPressure(pressure)

        val boundsAdjustedForPressure = RectF(
            point.x - rollingAveragePressure,
            point.y - rollingAveragePressure,
            point.x + rollingAveragePressure,
            point.y + rollingAveragePressure
        )
        strokesBounds.add(boundsAdjustedForPressure)

        val strokeColor = getColor(strokeColor, alpha)

        if (strokeColor == null) {
            alertModel.notifyBasicError((context as DrawingActivity)::finish)
            return
        }

        strokePaint.color = strokeColor

        val stroke = Stroke(
            mutableListOf(point.x),
            mutableListOf(point.y),
            mutableListOf(rollingAveragePressure),
            this.strokeColor,
            strokeAlpha.toFloat() / 255
        )

        drawing.strokes.add(stroke)
    }

    private fun approximateRollingAveragePressure(
        previousRollingAverage: Float,
        newPressure: Float
    ): Float {
        var newRollingAverage = previousRollingAverage

        newRollingAverage -= newRollingAverage / PRESSURE_SAMPLES_AVERAGED
        newRollingAverage += newPressure / PRESSURE_SAMPLES_AVERAGED

        return newRollingAverage
    }

    private fun lineTo(point: PointF, pressure: Float) {
        if (lastPoint.equals(
                Float.NaN,
                Float.NaN
            )
        ) { // if you start drawing after just erasing, and the pen was never lifted, this will compensate for it
            return moveTo(point, pressure)
        }

        val adjustedCurrentPressure = getAdjustedPressure(pressure)

        rollingAveragePressure =
            approximateRollingAveragePressure(rollingAveragePressure, adjustedCurrentPressure)
        updateLastStrokeBounds(point.x, point.y, rollingAveragePressure)

        strokePaint.strokeWidth = rollingAveragePressure

        strokePath.moveTo(
            lastPoint.x,
            lastPoint.y
        )

        strokePath.lineTo(
            point.x,
            point.y
        )

        tempCanvas.drawPath(strokePath, strokePaint)

        strokePath.reset()
        lastPoint.set(point)

        drawing.strokes.last { stroke ->
            stroke.pointsX.add(point.x)
            stroke.pointsY.add(point.y)
            stroke.pointsGirth.add(rollingAveragePressure)
        }
    }

    private fun eraseAtPoint(point: PointF) {
        when {
            erasePoints.first.x.isNaN() -> {
                erasePoints.first.set(point)
                return
            }
            erasePoints.second.x.isNaN() -> {
                erasePoints.second.set(point)
            }
            else -> {
                erasePoints.first.set(erasePoints.second)
                erasePoints.second.set(point)
            }
        }

        if (!lastPoint.equals(Float.NaN, Float.NaN)) {
            lastPoint.set(Float.NaN, Float.NaN)
        }

        val drawingClone = drawing.clone()
        var refreshScreen = false

        for (strokeIndex in drawingClone.strokes.size - 1 downTo 0) {
            val stroke = drawingClone.strokes[strokeIndex]
            var deleteStroke = false

            if (!doesEraserSegmentIntersectStroke(
                    erasePoints.first.x,
                    erasePoints.first.y,
                    erasePoints.second.x,
                    erasePoints.second.y,
                    strokeIndex
                )
            ) {
                continue
            }

            pointLoop@ for (pointIndex in 0..(stroke.pointsX.size - 2)) {
                if (pointIndex < stroke.pointsX.size - 1) {
                    var roundedPressure = stroke.pointsGirth[pointIndex].toInt()

                    if (roundedPressure < 5) {
                        roundedPressure = 5
                    }

                    for (pixel in 1..roundedPressure) {
                        val roundedPoint1 =
                            PointF(
                                stroke.pointsX[pointIndex].roundToInt().toFloat(),
                                stroke.pointsY[pointIndex].roundToInt().toFloat()
                            )
                        val roundedPoint2 =
                            PointF(
                                stroke.pointsX[pointIndex + 1].roundToInt().toFloat(),
                                stroke.pointsY[pointIndex + 1].roundToInt().toFloat()
                            )

                        val distBetweenErasePoints =
                            distanceBetweenPoints(erasePoints.first, erasePoints.second)
                        val distToFromRoundedPoint1 =
                            distanceBetweenPoints(erasePoints.first, roundedPoint1) +
                                distanceBetweenPoints(roundedPoint1, erasePoints.second)

                        if (((distToFromRoundedPoint1 - roundedPressure)..(distToFromRoundedPoint1 + roundedPressure)).contains(
                                distBetweenErasePoints
                            )
                        ) {
                            deleteStroke = true
                            break@pointLoop
                        }

                        val distToFromRoundedPoint2 =
                            distanceBetweenPoints(erasePoints.first, roundedPoint2) +
                                distanceBetweenPoints(roundedPoint2, erasePoints.second)

                        if (((distToFromRoundedPoint2 - roundedPressure)..(distToFromRoundedPoint2 + roundedPressure)).contains(
                                distBetweenErasePoints
                            )
                        ) {
                            deleteStroke = true
                            break@pointLoop
                        }

                        val distBetweenRoundedPoints =
                            distanceBetweenPoints(roundedPoint1, roundedPoint2)
                        val distToFromErasePoint1 =
                            distanceBetweenPoints(roundedPoint1, erasePoints.first) +
                                distanceBetweenPoints(erasePoints.first, roundedPoint2)

                        if (((distToFromErasePoint1 - roundedPressure)..(distToFromErasePoint1 + roundedPressure)).contains(
                                distBetweenRoundedPoints
                            )
                        ) {
                            deleteStroke = true
                            break@pointLoop
                        }

                        val distToFromErasePoint2 =
                            distanceBetweenPoints(roundedPoint1, erasePoints.second) +
                                distanceBetweenPoints(erasePoints.second, roundedPoint2)

                        if (((distToFromErasePoint2 - roundedPressure)..(distToFromErasePoint2 + roundedPressure)).contains(
                                distBetweenRoundedPoints
                            )
                        ) {
                            deleteStroke = true
                            break@pointLoop
                        }
                    }
                }
            }

            if (deleteStroke) {
                drawingClone.strokes.removeAt(strokeIndex)
                refreshScreen = true
            }
        }

        if (refreshScreen) {
            drawing = drawingClone
            strokesBounds.clear()
            tempCanvas.drawColor(
                Color.TRANSPARENT,
                PorterDuff.Mode.CLEAR
            )
            restoreFromModel()
        }
    }

    private fun distanceBetweenPoints(initialPoint: PointF, endPoint: PointF): Float =
        sqrt((initialPoint.x - endPoint.x).pow(2) + (initialPoint.y - endPoint.y).pow(2))

    fun setPenSize(penSize: Int) {
        penSizeMultiplier = penSize
    }

    fun startThread() {
        if (holder.surface.isValid && thread == null) {
            thread = Thread(this)
            isThreadAvailable = true
            thread?.start()
        }
    }

    fun stopThread() {
        if (thread == null) {
            return
        }
        isThreadAvailable = false
        while (thread?.isAlive == true) {
            try {
                thread?.join() ?: return
            } catch (e: Exception) {
            }
        }

        thread = null
    }

    override fun run() {
        while (isThreadAvailable && isDrawingAvailable) {
            if (holder == null) {
                return
            }

            var canvas: Canvas? = null
            try {
                canvas = if (Build.VERSION.SDK_INT > Build.VERSION_CODES.N_MR1) {
                    holder.lockHardwareCanvas()
                } else {
                    holder.lockCanvas()
                }
                render(canvas)
            } finally {
                holder.unlockCanvasAndPost(canvas)
            }
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (thread != null) {
            stopThread()
        }

        isThreadAvailable = true
        if (isDrawingAvailable) {
            startThread()
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {}

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        stopThread()
        holder.surface.release()
    }
}
