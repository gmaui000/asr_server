import io
import subprocess
import json
import time
from faster_whisper import WhisperModel
from flask import Flask, Response, render_template, request
from gevent import pywsgi
from tracker import QueryTracker

app = Flask("__name__")
app.config['JSON_AS_ASCII'] = False
# CORS(app)

# 初始化 WhisperModel
model = None

def init_whisper_model():
    global model
    if model is None:
        model = WhisperModel("./faster-whisper-large-v3/", device="cuda", compute_type="int8_float16")
        #model = WhisperModel("./faster-whisper-large-v3/", device="cpu", compute_type="int8")

# 在应用程序启动时调用初始化函数
init_whisper_model()

tracker = QueryTracker()

@app.route("/api/asr", methods=["POST"])
def api_asr():
    start = time.time()
    if "audio" not in request.files:
        return Response("No audio file provided\n", status=400)
    audio_file = request.files["audio"]
    audio_data = audio_file.read()

    vad = request.args.get("vad", type=bool, default=False)
    pts = request.args.get("pts", type=bool, default=False)
    thr = request.args.get("thr", type=float, default=0.5)

    # with open("audio.in", "wb") as f:
    #     f.write(audio_data)
    # try:
    #     subprocess.run(["ffmpeg", "-i", "audio.in", "-y", "-vn", "-ac", "1", "-ar", "16000", "in.wav"], check=True)
    # except subprocess.CalledProcessError as e:
    #     print("Conversion failed:", e)
    
    try:
        ffmpeg_process = subprocess.run(["ffmpeg", "-i", "-", "-y", "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", "pipe:1"], input=audio_data, capture_output=True, check=True)
    except subprocess.CalledProcessError as e:
        print("Conversion failed:", e)
        return Response("Not supported audio\n", status=400)

    output_audio = io.BytesIO(ffmpeg_process.stdout)
    # with open("in.wav", "wb") as f:
    #     f.write(output_audio.getvalue())

    segments, info = model.transcribe(output_audio, language="zh", beam_size=5, vad_filter=vad,  vad_parameters={"threshold": thr})

    print("Detected language '%s' with probability %f" % (info.language, info.language_probability))

    text = ""
    previous_text = ""
    for segment in segments:
        print("[%.2fs -> %.2fs] %s" % (segment.start, segment.end, segment.text))
        if segment.text != previous_text:
            if pts:
                text += "[%.2fs -> %.2fs]".format(segment.start, segment.end)
                text += segment.text + " "
            else:
                text += segment.text + " "
        previous_text = segment.text

    response_data = {"text": text}
    # with open("out.txt", "w", encoding="utf-8") as f:
    #     f.write(text)
    end = time.time()
    elapsed = round(end - start, 2)
    tracker.record_query(text, elapsed)

    json_data = json.dumps(response_data, ensure_ascii=False) + '\n'
    #return jsonify(response_data)
    return Response(json_data, mimetype="application/json")

@app.route("/demo")
def demo():
    return render_template("index.html")

@app.route("/")
def index():
    return Response(tracker.to_table_string(), mimetype='text/html')

if __name__ == "__main__":
    #server = pywsgi.WSGIServer(('0.0.0.0',40000),app,keyfile='certs/9002040_tts.cowarobot.com.key', certfile='certs/9002040_tts.cowarobot.com.crt')
    server = pywsgi.WSGIServer(('0.0.0.0',40005),app)
    server.serve_forever()
