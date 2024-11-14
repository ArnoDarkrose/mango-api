use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::manga::MangaRelation;
use super::{Error, Result};

use std::collections::HashMap;

pub trait Query: Serialize + std::fmt::Debug {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct EmptyQuery {}
impl Query for EmptyQuery {}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, std::hash::Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    Ab,
    Aa,
    Af,
    Ak,
    Sq,
    Am,
    Ar,
    An,
    Hy,
    Av,
    Ae,
    Ay,
    Az,
    Bm,
    Ba,
    Eu,
    Be,
    Bn,
    Bi,
    Bs,
    Br,
    Bg,
    My,
    Ca,
    Ch,
    Ce,
    Ny,
    Zh,
    Cu,
    Cv,
    Kw,
    Co,
    Cr,
    Hr,
    Cs,
    Da,
    Dv,
    Dz,
    En,
    Eo,
    Et,
    Ee,
    Fo,
    Fj,
    Fr,
    Fi,
    Fy,
    Ff,
    Gd,
    Gl,
    Lg,
    Ka,
    De,
    El,
    Kl,
    Gn,
    Gu,
    Hu,
    Ht,
    Ha,
    He,
    Hi,
    Ho,
    Is,
    Io,
    Ig,
    Id,
    Ia,
    Ie,
    Iu,
    Ik,
    Ga,
    It,
    Ja,
    Jv,
    Kn,
    Kr,
    Ks,
    Kk,
    Km,
    Ki,
    Rw,
    Ky,
    Kv,
    Kg,
    Ko,
    Kj,
    Ku,
    Lo,
    La,
    Lv,
    Li,
    Ln,
    Lt,
    Lu,
    Lb,
    Mk,
    Mg,
    Ms,
    Ml,
    Mt,
    Gv,
    Mi,
    Mr,
    Mh,
    Mn,
    Na,
    Nv,
    Nd,
    Nr,
    Ng,
    Ne,
    No,
    Nb,
    Nn,
    Nl,
    Oc,
    Oj,
    Or,
    Om,
    Os,
    Pi,
    Ps,
    Fa,
    Pl,
    Pt,
    Pa,
    Qu,
    Ro,
    Rm,
    Rn,
    Ru,
    Se,
    Sm,
    Sg,
    Sa,
    Sc,
    Sr,
    Sn,
    Sd,
    Si,
    Sk,
    Sl,
    So,
    St,
    Es,
    Su,
    Sw,
    Ss,
    Sv,
    Tl,
    Ty,
    Tg,
    Ta,
    Tt,
    Te,
    Th,
    Bo,
    Ti,
    To,
    Ts,
    Tn,
    Tr,
    Tk,
    Tw,
    Ug,
    Uk,
    Ur,
    Uz,
    Ve,
    Vi,
    Vo,
    Wa,
    Cy,
    Wo,
    Xh,
    Ii,
    Yi,
    Yo,
    Za,
    Zu,
    As,
    ZhHk,
    PtBr,
    EsLa,
    JaRo,
    KoRo,
    ZhRo,
}

pub type LocalizedString = HashMap<Locale, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Manga,
    CoverArt,
    Chapter,
    Author,
    ApiClient,
    ReportReason,
    ScanlationGroup,
    User,
    Tag,
    Artist,
    Creator,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Relationship {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub related: Option<MangaRelation>,
    pub attributes: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, std::hash::Hash)]
#[serde(rename_all = "camelCase")]
pub enum OrderOption {
    Title,
    Year,
    CreatedAt,
    UpdatedAt,
    LatestUploadedChapter,
    FollowedCount,
    Relevance,
    Chapter,
}

pub type SortingOptions = HashMap<OrderOption, Order>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ContentRating {
    Safe,
    Suggestive,
    Erotica,
    Pornographic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PublicationDemographic {
    Shounen,
    Shoujo,
    Josei,
    Seinen,
}

pub trait ResponseResultOk {
    fn response_result_ok(&self) -> Result<bool>;
}

impl ResponseResultOk for Value {
    fn response_result_ok(&self) -> Result<bool> {
        let result = match self.get("result") {
            Some(status) => status,
            None => return Err(Error::ParseError),
        };

        let responded_without_errors = if result.is_string() {
            let result = result.as_str().expect("verified to be a string");

            result == "ok"
        } else {
            return Err(Error::ParseError);
        };

        Ok(responded_without_errors)
    }
}
