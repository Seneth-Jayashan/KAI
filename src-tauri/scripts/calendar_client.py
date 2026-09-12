import json
import os
import requests
from icalendar import Calendar
from datetime import datetime
import pytz

CONFIG_PATH = os.path.expanduser("~/.local/share/kai/config.json")

def read_calendar():
    if not os.path.exists(CONFIG_PATH):
        print(f"Error: Config file not found at {CONFIG_PATH}")
        return

    with open(CONFIG_PATH, "r") as f:
        config = json.load(f)

    ical_url = config.get("calendar", {}).get("google_ical_url")
    if not ical_url or "YOUR_" in ical_url:
        print("Error: Google iCal URL is not set in config.json")
        return

    try:
        response = requests.get(ical_url)
        response.raise_for_status()
        
        cal = Calendar.from_ical(response.content)
        
        now = datetime.now(pytz.utc)
        upcoming_events = []

        for component in cal.walk():
            if component.name == "VEVENT":
                start_dt = component.get('dtstart').dt
                
                # Handle dates without time
                if not isinstance(start_dt, datetime):
                    start_dt = datetime.combine(start_dt, datetime.min.time()).replace(tzinfo=pytz.utc)
                
                if start_dt.tzinfo is None:
                    start_dt = start_dt.replace(tzinfo=pytz.utc)
                
                if start_dt >= now:
                    summary = str(component.get('summary'))
                    upcoming_events.append((start_dt, summary))

        upcoming_events.sort(key=lambda x: x[0])

        print("--- UPCOMING GOOGLE CALENDAR EVENTS ---")
        if not upcoming_events:
            print("No upcoming events found.")
        else:
            # show next 10 events
            for dt, summary in upcoming_events[:10]:
                local_dt = dt.astimezone() # convert to local time
                print(f"{local_dt.strftime('%a, %b %d at %I:%M %p')} - {summary}")

    except Exception as e:
        print(f"Error fetching calendar: {e}")

if __name__ == "__main__":
    read_calendar()
