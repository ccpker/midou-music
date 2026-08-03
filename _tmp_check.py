import time
now = int(time.time())
exp = 1784970145
print(f"current: {now}, token_expires: {exp}, delta_days: {(exp - now) / 86400:.1f}")
