import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

df_tmp = pd.read_csv("stock_evolution.csv")
df = df_tmp.iloc[::1, :].reset_index(drop=True) # to read 1/x lines

stocks = df.columns[1:]
times = df['time'].values

fig, ax = plt.subplots()
bars = ax.bar(stocks, df.iloc[0, 1:])

ax.set_ylim(0, df.iloc[:, 1:].max().max() * 1.1)
ax.set_xticklabels(stocks, rotation=45, ha='right')

def update(frame):
    for bar, h in zip(bars, df.iloc[frame, 1:]):
        bar.set_height(h)
    ax.set_title(f"Time: {df.iloc[frame, 0]}")
    return bars

ani = FuncAnimation(fig, update, frames=len(df), interval=1, blit=False)

plt.show()
