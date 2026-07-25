import gzip
import json
import smbclient
import random

x = 1
items_found = 0
lines_to_skip = random.randint(5000, 20000)

# 1. YOU MUST REGISTER THE SESSION WITH YOUR PI'S CREDENTIALS
# Replace "pi" and "your_password" with the actual SMB username and password for the share.
smbclient.register_session(
    "raspberrypi.local", username="username", password="password"
)
share_path = r"\\raspberrypi.local\rpi-storage\wikipedia\latest-all.json.gz"

with smbclient.open_file(share_path, mode="rb") as remote_file:
    with gzip.open(remote_file, "rt", encoding="utf-8") as f:
        for line in f:
            clean_line = line.strip().rstrip(",")

            if clean_line in ("[", "]") or not clean_line:
                continue

            # If we haven't skipped enough lines yet, keep going
            if lines_to_skip > 0:
                lines_to_skip -= 1
                continue

            # If we reach this point, we've skipped the required amount!
            data = json.loads(clean_line)

            print(f"[{items_found + 1}/{x}] Successfully loaded item: {data.get('id')}")
            print(json.dumps(data, indent=2))  # Uncomment to see the full object

            items_found += 1

            # Stop once we have our x items
            if items_found >= x:
                break

            # Reset the skip counter for the next random item
            lines_to_skip = random.randint(5000, 20000)
print("Finished sampling!")
