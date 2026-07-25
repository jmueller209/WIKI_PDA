import json
import tqdm

with open("./../P_IDs.json", "r") as f:
    data = json.load(f)

data_types = []
for i in tqdm.tqdm(range(len(data)), desc="Inspecting P_IDs.json"):
    data_type = data[i].get("datatype")
    if data_type not in data_types:
        data_types.append(data_type)


print("Unique data types found in P_IDs.json:")
for data_type in data_types:
    print(data_type)
