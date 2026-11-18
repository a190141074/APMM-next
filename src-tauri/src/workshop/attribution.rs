//! 工坊公开资料的署名和链接约束，投稿复用此契约，不接触本机候选资料或凭据。

/// 校验公开署名及链接；空资料兼容旧包，链接只允许无凭据的 HTTPS 地址。
pub(crate) fn validate(
    author: &str,
    artist: &str,
    author_urls: &[String],
    artist_urls: &[String],
    source_url: &str,
) -> Result<(), String> {
    if author.chars().count() > 120 || artist.chars().count() > 120 {
        return Err("作者和画师署名最多 120 个字符".into());
    }
    if author_urls.len() > 8 || artist_urls.len() > 8 {
        return Err("作者和画师各最多填写 8 个链接".into());
    }
    for link in author_urls
        .iter()
        .chain(artist_urls)
        .map(String::as_str)
        .chain((!source_url.is_empty()).then_some(source_url))
    {
        let url = reqwest::Url::parse(link).map_err(|_| "链接须为 HTTPS 网址")?;
        if link.len() > 2048
            || link != link.trim()
            || link.chars().any(char::is_control)
            || url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("链接须为不含登录凭据的 HTTPS 网址，且最多 2048 字节".into());
        }
    }
    Ok(())
}
