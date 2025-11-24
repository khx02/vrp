# import pandas as pd
# import matplotlib.pyplot as plt

# # Load CSV
# df = pd.read_csv("best_so_far.csv")

# # Plot
# plt.plot(df["iteration"], df["new_best_so_far"], marker="o", linestyle="-", color="b")

# Labels and Title
# plt.xlabel("Iteration")
# plt.ylabel("Best So Far")
# plt.title("Best So Far Over Iterations")

# # Show Grid
# plt.grid(True)

# # Save or Show Plot
# plt.savefig("best_so_far_plot.png")  # Save as PNG (optional)
# plt.show()

import pandas as pd
import matplotlib.pyplot as plt

# Load CSV
df = pd.read_csv("best_so_far.csv")

# Extract the early stopping iteration (take the first row's value)
ended_early_iteration = df["ended_early_iteration"].iloc[
    0
]  # Since all values are the same, take any

# Plot main line
plt.plot(
    df["iteration"],
    df["new_best_so_far"],
    marker="o",
    linestyle="-",
    color="b",
    label="Best So Far",
)

# Plot a vertical line at early stopping iteration
plt.axvline(
    x=ended_early_iteration,
    color="r",
    linestyle="--",
    label=f"Early Stop (Iteration {ended_early_iteration})",
)

# Labels and Title
plt.xlabel("Iteration")
plt.ylabel("Best So Far")
plt.title("Best So Far Over Iterations with Early Stop Marker")

# Show Grid and Legend
plt.grid(True)
plt.legend()

# Save or Show Plot
plt.savefig("best_so_far_plot.png")  # Save as PNG (optional)
plt.show()
