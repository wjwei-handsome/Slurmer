from tqdm import tqdm
import time
import sys

desc = sys.argv[1]
for i in tqdm(range(10000), desc=desc):
    time.sleep(0.05)
