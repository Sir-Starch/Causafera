use causafera_types::{
    AgentId, AgreementId, AgreementPartyId, AuthorityGrantId, AuthorityScopeId,
    CommunicationChannelId, CommunicationLinkId, DocumentId, EntityId, InstitutionalRuleId,
    OrganizationId, PracticeId, PropertyClaimId, RoleAssignmentId, RoleSchemaId, SimulationTime,
    SocialRelationId, SocialRelationSchemaId, TraceId,
};

pub const MAX_ORGANIZATIONS: usize = 4_096;
pub const MAX_SOCIAL_RELATIONS: usize = 65_536;
pub const MAX_ROLE_ASSIGNMENTS: usize = 32_768;
pub const MAX_COMMUNICATION_LINKS: usize = 65_536;
pub const MAX_AUTHORITY_GRANTS: usize = 16_384;
pub const MAX_PROPERTY_CLAIMS: usize = 16_384;
pub const MAX_INSTITUTIONAL_RULES: usize = 8_192;
pub const MAX_ORGANIZATION_PRACTICES: usize = 16_384;
pub const MAX_AGREEMENTS: usize = 8_192;
pub const MAX_RULE_REFERENCES: usize = 64;
pub const MAX_AGREEMENT_PARTIES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocialRelation {
    pub id: SocialRelationId,
    pub source: AgentId,
    pub target: AgentId,
    pub schema: SocialRelationSchemaId,
    pub strength: i32,
    pub established_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleAssignment {
    pub id: RoleAssignmentId,
    pub organization: OrganizationId,
    pub member: AgentId,
    pub role: RoleSchemaId,
    pub assigned_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommunicationLink {
    pub id: CommunicationLinkId,
    pub organization: Option<OrganizationId>,
    pub source: AgentId,
    pub target: AgentId,
    pub channel: CommunicationChannelId,
    pub capacity: u32,
    pub delay_ticks: u32,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthorityGrant {
    pub id: AuthorityGrantId,
    pub organization: OrganizationId,
    pub holder: RoleAssignmentId,
    pub scope: AuthorityScopeId,
    pub weight: u32,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyClaim {
    pub id: PropertyClaimId,
    pub organization: OrganizationId,
    pub claimant: RoleAssignmentId,
    pub object: EntityId,
    pub strength: u32,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrganizationPractice {
    pub organization: OrganizationId,
    pub practice: PracticeId,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstitutionalRule {
    pub id: InstitutionalRuleId,
    pub organization: OrganizationId,
    pub text: DocumentId,
    pub interpretations: Vec<DocumentId>,
    pub precedents: Vec<DocumentId>,
    pub authorities: Vec<AuthorityGrantId>,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttestedAgreement {
    pub id: AgreementId,
    pub text: DocumentId,
    pub parties: Vec<AgreementPartyId>,
    pub witnesses: Vec<AgreementPartyId>,
    pub authorities: Vec<AuthorityGrantId>,
    pub formed_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SocialState {
    organizations: Vec<OrganizationId>,
    relations: Vec<SocialRelation>,
    roles: Vec<RoleAssignment>,
    communication: Vec<CommunicationLink>,
    authorities: Vec<AuthorityGrant>,
    property_claims: Vec<PropertyClaim>,
    rules: Vec<InstitutionalRule>,
    practices: Vec<OrganizationPractice>,
    agreements: Vec<AttestedAgreement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocialError {
    CapacityExceeded,
    DuplicateId,
    DuplicateReference,
    EmptyParties,
    SelfRelation,
    UnknownOrganization,
    UnknownRoleAssignment,
    UnknownAuthorityGrant,
    CrossOrganizationReference,
}

impl SocialState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut organizations: Vec<OrganizationId>,
        mut relations: Vec<SocialRelation>,
        mut roles: Vec<RoleAssignment>,
        mut communication: Vec<CommunicationLink>,
        mut authorities: Vec<AuthorityGrant>,
        mut property_claims: Vec<PropertyClaim>,
        mut rules: Vec<InstitutionalRule>,
        mut practices: Vec<OrganizationPractice>,
        mut agreements: Vec<AttestedAgreement>,
    ) -> Result<Self, SocialError> {
        check_capacities(
            &organizations,
            &relations,
            &roles,
            &communication,
            &authorities,
            &property_claims,
            &rules,
            &practices,
            &agreements,
        )?;

        organizations.sort_unstable();
        reject_adjacent_duplicates(&organizations)?;
        relations.sort_unstable_by_key(|record| record.id);
        roles.sort_unstable_by_key(|record| record.id);
        communication.sort_unstable_by_key(|record| record.id);
        authorities.sort_unstable_by_key(|record| record.id);
        property_claims.sort_unstable_by_key(|record| record.id);
        rules.sort_unstable_by_key(|record| record.id);
        practices.sort_unstable();
        agreements.sort_unstable_by_key(|record| record.id);

        reject_duplicate_keys(&relations, |record| record.id)?;
        reject_duplicate_keys(&roles, |record| record.id)?;
        reject_duplicate_keys(&communication, |record| record.id)?;
        reject_duplicate_keys(&authorities, |record| record.id)?;
        reject_duplicate_keys(&property_claims, |record| record.id)?;
        reject_duplicate_keys(&rules, |record| record.id)?;
        reject_duplicate_keys(&agreements, |record| record.id)?;
        reject_duplicate_practices(&practices)?;

        for relation in &relations {
            if relation.source == relation.target {
                return Err(SocialError::SelfRelation);
            }
        }
        for role in &roles {
            require_organization(&organizations, role.organization)?;
        }
        for link in &communication {
            if let Some(organization) = link.organization {
                require_organization(&organizations, organization)?;
            }
        }
        for grant in &authorities {
            require_organization(&organizations, grant.organization)?;
            let role = find_role(&roles, grant.holder)?;
            if role.organization != grant.organization {
                return Err(SocialError::CrossOrganizationReference);
            }
        }
        for claim in &property_claims {
            require_organization(&organizations, claim.organization)?;
            let role = find_role(&roles, claim.claimant)?;
            if role.organization != claim.organization {
                return Err(SocialError::CrossOrganizationReference);
            }
        }
        for practice in &practices {
            require_organization(&organizations, practice.organization)?;
        }
        for rule in &mut rules {
            require_organization(&organizations, rule.organization)?;
            canonicalize_rule(rule)?;
            for authority in &rule.authorities {
                let grant = find_authority(&authorities, *authority)?;
                if grant.organization != rule.organization {
                    return Err(SocialError::CrossOrganizationReference);
                }
            }
        }
        for agreement in &mut agreements {
            canonicalize_agreement(agreement)?;
            for authority in &agreement.authorities {
                find_authority(&authorities, *authority)?;
            }
        }

        Ok(Self {
            organizations,
            relations,
            roles,
            communication,
            authorities,
            property_claims,
            rules,
            practices,
            agreements,
        })
    }

    pub fn organizations(&self) -> &[OrganizationId] {
        &self.organizations
    }
    pub fn relations(&self) -> &[SocialRelation] {
        &self.relations
    }
    pub fn roles(&self) -> &[RoleAssignment] {
        &self.roles
    }
    pub fn communication(&self) -> &[CommunicationLink] {
        &self.communication
    }
    pub fn authorities(&self) -> &[AuthorityGrant] {
        &self.authorities
    }
    pub fn property_claims(&self) -> &[PropertyClaim] {
        &self.property_claims
    }
    pub fn rules(&self) -> &[InstitutionalRule] {
        &self.rules
    }
    pub fn practices(&self) -> &[OrganizationPractice] {
        &self.practices
    }
    pub fn agreements(&self) -> &[AttestedAgreement] {
        &self.agreements
    }
}

#[allow(clippy::too_many_arguments)]
fn check_capacities(
    organizations: &[OrganizationId],
    relations: &[SocialRelation],
    roles: &[RoleAssignment],
    communication: &[CommunicationLink],
    authorities: &[AuthorityGrant],
    property_claims: &[PropertyClaim],
    rules: &[InstitutionalRule],
    practices: &[OrganizationPractice],
    agreements: &[AttestedAgreement],
) -> Result<(), SocialError> {
    if organizations.len() > MAX_ORGANIZATIONS
        || relations.len() > MAX_SOCIAL_RELATIONS
        || roles.len() > MAX_ROLE_ASSIGNMENTS
        || communication.len() > MAX_COMMUNICATION_LINKS
        || authorities.len() > MAX_AUTHORITY_GRANTS
        || property_claims.len() > MAX_PROPERTY_CLAIMS
        || rules.len() > MAX_INSTITUTIONAL_RULES
        || practices.len() > MAX_ORGANIZATION_PRACTICES
        || agreements.len() > MAX_AGREEMENTS
    {
        return Err(SocialError::CapacityExceeded);
    }
    Ok(())
}

fn reject_adjacent_duplicates<T: Eq>(values: &[T]) -> Result<(), SocialError> {
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(SocialError::DuplicateId)
    } else {
        Ok(())
    }
}

fn reject_duplicate_keys<T, K: Eq + Copy>(
    values: &[T],
    key: impl Fn(&T) -> K,
) -> Result<(), SocialError> {
    if values.windows(2).any(|pair| key(&pair[0]) == key(&pair[1])) {
        Err(SocialError::DuplicateId)
    } else {
        Ok(())
    }
}

fn reject_duplicate_practices(values: &[OrganizationPractice]) -> Result<(), SocialError> {
    if values.windows(2).any(|pair| {
        (pair[0].organization, pair[0].practice) == (pair[1].organization, pair[1].practice)
    }) {
        Err(SocialError::DuplicateId)
    } else {
        Ok(())
    }
}

fn require_organization(values: &[OrganizationId], id: OrganizationId) -> Result<(), SocialError> {
    values
        .binary_search(&id)
        .map(|_| ())
        .map_err(|_| SocialError::UnknownOrganization)
}

fn find_role(
    values: &[RoleAssignment],
    id: RoleAssignmentId,
) -> Result<&RoleAssignment, SocialError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|index| &values[index])
        .map_err(|_| SocialError::UnknownRoleAssignment)
}

fn find_authority(
    values: &[AuthorityGrant],
    id: AuthorityGrantId,
) -> Result<&AuthorityGrant, SocialError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|index| &values[index])
        .map_err(|_| SocialError::UnknownAuthorityGrant)
}

fn sort_unique_bounded<T: Ord>(values: &mut [T], maximum: usize) -> Result<(), SocialError> {
    if values.len() > maximum {
        return Err(SocialError::CapacityExceeded);
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SocialError::DuplicateReference);
    }
    Ok(())
}

fn canonicalize_rule(rule: &mut InstitutionalRule) -> Result<(), SocialError> {
    sort_unique_bounded(&mut rule.interpretations, MAX_RULE_REFERENCES)?;
    sort_unique_bounded(&mut rule.precedents, MAX_RULE_REFERENCES)?;
    sort_unique_bounded(&mut rule.authorities, MAX_RULE_REFERENCES)
}

fn canonicalize_agreement(agreement: &mut AttestedAgreement) -> Result<(), SocialError> {
    if agreement.parties.is_empty() {
        return Err(SocialError::EmptyParties);
    }
    sort_unique_bounded(&mut agreement.parties, MAX_AGREEMENT_PARTIES)?;
    sort_unique_bounded(&mut agreement.witnesses, MAX_AGREEMENT_PARTIES)?;
    sort_unique_bounded(&mut agreement.authorities, MAX_RULE_REFERENCES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(reverse: bool) -> SocialState {
        let org = OrganizationId::new(1);
        let mut roles = vec![
            RoleAssignment {
                id: RoleAssignmentId::new(2),
                organization: org,
                member: AgentId::new(20),
                role: RoleSchemaId::new(8),
                assigned_at: SimulationTime::new(3),
                trace: TraceId::new(2),
            },
            RoleAssignment {
                id: RoleAssignmentId::new(1),
                organization: org,
                member: AgentId::new(10),
                role: RoleSchemaId::new(7),
                assigned_at: SimulationTime::new(2),
                trace: TraceId::new(1),
            },
        ];
        if !reverse {
            roles.reverse();
        }
        let authorities = vec![AuthorityGrant {
            id: AuthorityGrantId::new(1),
            organization: org,
            holder: RoleAssignmentId::new(1),
            scope: AuthorityScopeId::new(4),
            weight: 3,
            trace: TraceId::new(3),
        }];
        SocialState::new(
            vec![org],
            vec![SocialRelation {
                id: SocialRelationId::new(1),
                source: AgentId::new(10),
                target: AgentId::new(20),
                schema: SocialRelationSchemaId::new(6),
                strength: -2,
                established_at: SimulationTime::new(1),
                trace: TraceId::new(1),
            }],
            roles,
            vec![CommunicationLink {
                id: CommunicationLinkId::new(1),
                organization: Some(org),
                source: AgentId::new(10),
                target: AgentId::new(20),
                channel: CommunicationChannelId::new(9),
                capacity: 4,
                delay_ticks: 2,
                trace: TraceId::new(4),
            }],
            authorities,
            vec![PropertyClaim {
                id: PropertyClaimId::new(1),
                organization: org,
                claimant: RoleAssignmentId::new(2),
                object: EntityId::new(90),
                strength: 2,
                trace: TraceId::new(5),
            }],
            vec![InstitutionalRule {
                id: InstitutionalRuleId::new(1),
                organization: org,
                text: DocumentId::new(1),
                interpretations: vec![DocumentId::new(3), DocumentId::new(2)],
                precedents: vec![DocumentId::new(5)],
                authorities: vec![AuthorityGrantId::new(1)],
                trace: TraceId::new(6),
            }],
            vec![OrganizationPractice {
                organization: org,
                practice: PracticeId::new(1),
                trace: TraceId::new(7),
            }],
            vec![AttestedAgreement {
                id: AgreementId::new(1),
                text: DocumentId::new(6),
                parties: vec![AgreementPartyId::new(2), AgreementPartyId::new(1)],
                witnesses: vec![],
                authorities: vec![AuthorityGrantId::new(1)],
                formed_at: SimulationTime::new(9),
                trace: TraceId::new(8),
            }],
        )
        .unwrap()
    }

    #[test]
    fn construction_is_input_order_independent_and_canonical() {
        let left = state(false);
        let right = state(true);
        assert_eq!(left, right);
        assert_eq!(left.roles()[0].id, RoleAssignmentId::new(1));
        assert_eq!(
            left.rules()[0].interpretations,
            vec![DocumentId::new(2), DocumentId::new(3)]
        );
        assert_eq!(
            left.agreements()[0].parties,
            vec![AgreementPartyId::new(1), AgreementPartyId::new(2)]
        );
    }

    #[test]
    fn organization_has_no_aggregate_cognitive_state() {
        let state = state(false);
        assert_eq!(state.organizations(), &[OrganizationId::new(1)]);
        assert_eq!(state.roles().len(), 2);
        assert_eq!(state.communication().len(), 1);
    }

    #[test]
    fn cross_organization_authority_is_rejected() {
        let mut state = state(false);
        let mut rules = state.rules.clone();
        rules[0].organization = OrganizationId::new(2);
        state.organizations.push(OrganizationId::new(2));
        assert_eq!(
            SocialState::new(
                state.organizations,
                state.relations,
                state.roles,
                state.communication,
                state.authorities,
                state.property_claims,
                rules,
                state.practices,
                state.agreements
            ),
            Err(SocialError::CrossOrganizationReference)
        );
    }

    #[test]
    fn duplicate_parties_and_self_relations_are_rejected() {
        let mut agreement = state(false).agreements[0].clone();
        agreement.parties = vec![AgreementPartyId::new(1), AgreementPartyId::new(1)];
        assert_eq!(
            canonicalize_agreement(&mut agreement),
            Err(SocialError::DuplicateReference)
        );
        let relation = SocialRelation {
            id: SocialRelationId::new(1),
            source: AgentId::new(1),
            target: AgentId::new(1),
            schema: SocialRelationSchemaId::new(1),
            strength: 1,
            established_at: SimulationTime::new(0),
            trace: TraceId::new(1),
        };
        assert_eq!(
            SocialState::new(
                vec![],
                vec![relation],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![]
            ),
            Err(SocialError::SelfRelation)
        );
    }
}
