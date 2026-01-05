#!/usr/bin/env bash
# Runs benchmark and formats the output.

cargo bench --bench teeny_vec --quiet -- --quick \
    | grep -E 'time:|^push|^clone' \
    | sed '/^ /!s/ *time:/\ntime:/;s/^ *time:/time:/' \
    | sed 'N;s/\n/\t/' \
    | awk -F'\t' '{ printf "%-40s %s\n", $1, $2}' \
    | gawk '
{
    # Parse input: push_small_inline/TeenyVec/1
    split($1, parts, "/")
    group = parts[1]
    impl  = parts[2]
    size  = parts[3]
    time  = $5

    # Handle the "clone" format which uses underscores
    if (size == "") {
        split(parts[2], subparts, "_")
        impl = subparts[1]
        size = subparts[3]
    }

    # Store data
    times[group][impl][size] = time
    groups[group] = 1
    impls[group][impl] = 1

    # Track unique sizes per group and find min time for relative diff
    sizes[group][size] = 1
    if (!(size in min_time[group]) || time < min_time[group][size]) {
        min_time[group][size] = time
    }
}
END {
    for (g in groups) {
        # 1. Identify and sort unique sizes for this group
        n_count = 0; for (s in sizes[g]) sorted_sizes[++n_count] = s
        asort(sorted_sizes, sorted_sizes, "@val_num_asc")

        # 2. Print Header
        printf "### %s\n\n", g
        printf "| %s", "Implementation"
        for (i=1; i<=n_count; i++) printf " | N=%s", sorted_sizes[i]
        printf " | Overall |\n"

        printf "| ---"
        for (i=1; i<=n_count; i++) printf " | ---"
        printf " | --- |\n"

        # 3. Print Rows
        for (im in impls[g]) {
            wins = 0
            printf "| %s", im
            for (i=1; i<=n_count; i++) {
                sz = sorted_sizes[i]
                t = times[g][im][sz]

                if (t == min_time[g][sz]) {
                    printf " | 🏆%s ", sprintf("(%.1fns)", t)
                    wins++
                } else if (t > 0) {
                    diff = (t / min_time[g][sz] - 1) * 100
                    printf " | %s ", sprintf("+%.1f%%", diff)
                } else {
                    printf " | %s", "-"
                }
            }
            printf " | %s |\n", (wins > 0 ? "🏆 x " wins : "-")
        }

        printf "| ---"
        for (i=1; i<=n_count; i++) printf " | ---"
        printf " | --- |\n"

        delete sorted_sizes
    }
}'
