use crate::file_writer::BBFiles;

pub trait TraceBuilder {
    fn create_trace_builder_hpp(
        &mut self,
        name: &str,
        fixed: &[String],
        shifted: &[String],
    ) -> String;
}

fn trace_cpp_includes(relation_path: &str, name: &str) -> String {
    let boilerplate = r#"
#include "barretenberg/ecc/curves/bn254/fr.hpp"
#include <cstdint>
#include <fstream>
#include <iostream>
#include <string>
#include <sys/types.h>
#include <vector>
#include "barretenberg/proof_system/arithmetization/arithmetization.hpp"
"#
    .to_owned();

    format!(
        "
{boilerplate}
#include \"barretenberg/{relation_path}/{name}.hpp\"
#include \"barretenberg/proof_system/arithmetization/generated/{name}_arith.hpp\"
"
    )
}

fn trace_hpp_includes(name: &str) -> String {
    format!(
        "
    // AUTOGENERATED FILE
    #pragma once

    #include \"barretenberg/common/throw_or_abort.hpp\"
    #include \"barretenberg/ecc/curves/bn254/fr.hpp\"
    #include \"barretenberg/proof_system/arithmetization/arithmetization.hpp\"
    #include \"barretenberg/proof_system/circuit_builder/circuit_builder_base.hpp\"
    
    #include \"barretenberg/honk/flavor/generated/{name}_flavor.hpp\"
    #include \"barretenberg/proof_system/arithmetization/generated/{name}_arith.hpp\"
    #include \"barretenberg/proof_system/relations/generated/{name}.hpp\"
"
    )
}

impl TraceBuilder for BBFiles {
    // Create trace builder
    // Generate some code that can read a commits.bin and constants.bin into data structures that bberg understands
    fn create_trace_builder_hpp(
        &mut self,
        name: &str,
        all_cols: &[String],
        to_be_shifted: &[String],
    ) -> String {
        let includes = trace_hpp_includes(name);

        let num_polys = all_cols.len();
        let num_cols = all_cols.len() + to_be_shifted.len();

        let compute_polys_assignemnt = all_cols
            .iter()
            .map(|name| format!("polys.{name}[i] = rows[i].{name};",))
            .collect::<Vec<String>>()
            .join("\n");

        let all_poly_shifts = &to_be_shifted
            .iter()
            .map(|name| format!("polys.{name}_shift = Polynomial(polys.{name}.shifted());"))
            .collect::<Vec<String>>()
            .join("\n");

        format!("
{includes}

using namespace barretenberg;

namespace proof_system {{

class {name}TraceBuilder {{
    public:
        using FF = arithmetization::{name}Arithmetization::FF;
        using Row = {name}_vm::Row<FF>;

        // TODO: tempalte
        using Polynomial = honk::flavor::{name}Flavor::Polynomial;
        using AllPolynomials = honk::flavor::{name}Flavor::AllPolynomials;

        static constexpr size_t num_fixed_columns = {num_cols};
        static constexpr size_t num_polys = {num_polys};
        std::vector<Row> rows;

        [[maybe_unused]] void build_circuit();

        AllPolynomials compute_polynomials() {{
            const auto num_rows = get_circuit_subgroup_size();
            AllPolynomials polys;

            // Allocate mem for each column
            for (size_t i = 0; i < num_fixed_columns; ++i) {{
                polys[i] = Polynomial(num_rows);
            }}

            for (size_t i = 0; i < rows.size(); i++) {{
                {compute_polys_assignemnt}
            }}

            {all_poly_shifts }

            return polys;
        }}

        [[maybe_unused]] bool check_circuit() {{
            auto polys = compute_polynomials();
            const size_t num_rows = polys[0].size();

            const auto evaluate_relation = [&]<typename Relation>(const std::string& relation_name) {{
                typename Relation::ArrayOfValuesOverSubrelations result;
                for (auto& r : result) {{
                    r = 0;
                }}
                constexpr size_t NUM_SUBRELATIONS = result.size();

                for (size_t i = 0; i < num_rows; ++i) {{
                    Relation::accumulate(result, polys.get_row(i), {{}}, 1);

                    bool x = true;
                    for (size_t j = 0; j < NUM_SUBRELATIONS; ++j) {{
                        if (result[j] != 0) {{
                            throw_or_abort(format(\"Relation \", relation_name, \", subrelation index \", j, \" failed at row \", i));
                            x = false;
                        }}
                    }}
                    if (!x) {{
                        return false;
                    }}
                }}
                return true;
            }};

            return evaluate_relation.template operator()<{name}_vm::{name}<FF>>(\"{name}\");
        }}

        [[nodiscard]] size_t get_num_gates() const {{ return rows.size(); }}

        [[nodiscard]] size_t get_circuit_subgroup_size() const
        {{
            const size_t num_rows = get_num_gates();
            const auto num_rows_log2 = static_cast<size_t>(numeric::get_msb64(num_rows));
            size_t num_rows_pow2 = 1UL << (num_rows_log2 + (1UL << num_rows_log2 == num_rows ? 0 : 1));
            return num_rows_pow2;
        }}


}};
}}
        ")
    }
}