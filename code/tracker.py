from datetime import datetime
from collections import deque

class QueryRecord:
    def __init__(self, text, duration):
        self.text = text
        self.duration = duration

class QueryTracker:
    def __init__(self):
        self.total_cnts = 0
        self.start_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        self.records = deque(maxlen=10)

    def record_query(self, text, duration):
        record = QueryRecord(text, duration)
        self.records.append(record)
        self.total_cnts += 1

    def get_total_cnts(self):
        return self.total_cnts

    def get_records(self):
        return list(self.records)

    def to_table_string(self):
        html_table = "<h1>Total Query Times: {}</h1>".format(self.total_cnts)
        html_table += "<p>Last 10 Queries:</p>"
        html_table += "<p>| Index | Duration(s) | Text |</p>"
        for index, record in enumerate(self.records):
            html_table += "<p>| {:02d} | {:.2f} | {} |</p>".format(index, record.duration, record.text)
        
        return f"{html_table}"
