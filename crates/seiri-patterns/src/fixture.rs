use seiri_core::PatternGroup;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PatternNegativeFixture {
    pub id: &'static str,
    pub group: PatternGroup,
    pub repository: &'static str,
    pub pattern_id: &'static str,
}

const COMMON_NEGATIVE_FIXTURES: [PatternNegativeFixture; 13] = [
    negative_fixture(
        "negative.idn.missing_readme",
        PatternGroup::Idn,
        "missing-readme-repo",
        "common.identity.readme_present",
    ),
    negative_fixture(
        "negative.doc.minimal",
        PatternGroup::Doc,
        "minimal-repo",
        "common.docs.route_present",
    ),
    negative_fixture(
        "negative.qst.minimal",
        PatternGroup::Qst,
        "minimal-repo",
        "common.quickstart.route_present",
    ),
    negative_fixture(
        "negative.sup.minimal",
        PatternGroup::Sup,
        "minimal-repo",
        "common.support.route_present",
    ),
    negative_fixture(
        "negative.sec.minimal",
        PatternGroup::Sec,
        "minimal-repo",
        "common.security.route_present",
    ),
    negative_fixture(
        "negative.ctr.minimal",
        PatternGroup::Ctr,
        "minimal-repo",
        "common.contributing.route_present",
    ),
    negative_fixture(
        "negative.int.minimal",
        PatternGroup::Int,
        "minimal-repo",
        "INT-002",
    ),
    negative_fixture(
        "negative.aut.minimal",
        PatternGroup::Aut,
        "minimal-repo",
        "common.automation.route_present",
    ),
    negative_fixture(
        "negative.rel.minimal",
        PatternGroup::Rel,
        "minimal-repo",
        "common.release.route_present",
    ),
    negative_fixture(
        "negative.own.minimal",
        PatternGroup::Own,
        "minimal-repo",
        "OWN-001",
    ),
    negative_fixture(
        "negative.gov.minimal",
        PatternGroup::Gov,
        "minimal-repo",
        "GOV-001",
    ),
    negative_fixture(
        "negative.hyg.minimal",
        PatternGroup::Hyg,
        "minimal-repo",
        "HYG-001",
    ),
    negative_fixture(
        "negative.lif.minimal",
        PatternGroup::Lif,
        "minimal-repo",
        "common.license.file_present",
    ),
];

#[must_use]
pub fn common_negative_fixtures() -> Vec<PatternNegativeFixture> {
    COMMON_NEGATIVE_FIXTURES.to_vec()
}

const fn negative_fixture(
    id: &'static str,
    group: PatternGroup,
    repository: &'static str,
    pattern_id: &'static str,
) -> PatternNegativeFixture {
    PatternNegativeFixture {
        id,
        group,
        repository,
        pattern_id,
    }
}
